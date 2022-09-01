# Function

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/04_function.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/04_function.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

A function is a block that takes an "argument", processes it, and returns it as a "return value". It is defined as follows.

```erg
add x, y = x + y
# or
add(x, y) = x + y
```

The names specified after a function name are called parameters.
In contrast, the objects passed to a function are called arguments.
The function `add` is a function that takes `x` and `y` as parameters and returns the sum of them, `x + y`.
The defined function can be called (applied/invoked) as follows.

```erg
add 1, 2
# or
add(1, 2)
```

## Colon application style

Functions are invoked like `f x, y, ...`, but if there are too many arguments for a single line, they can be applied using `:` (colon).

```erg
f some_long_name_variable_1 + some_long_name_variable_2, some_long_name_variable_3 * some_long_name_variable_4
```

```erg
f some_long_name_variable_1 + some_long_name_variable_2:
    some_long_name_variable_3 * some_long_name_variable_4
```

```erg
f:
    some_long_name_variable_1 + some_long_name_variable_2
    some_long_name_variable_3 * some_long_name_variable_4
```

All three codes above mean the same thing. This style is also useful when using `if` functions, for example.

```erg
result = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
```

After `:`, no code other than comments may be written, and must always be on a new line.

## Keyword Arguments

If a function is defined with a large number of parameters, there is a danger of passing the arguments in the wrong order.
In such cases, it is safe to call the function using keyword arguments.

```erg
f x, y, z, w, v, u: Int = ...
```

The functions defined above have many arguments and are arranged in a confusing order. You should not create such a function, but you may encounter such code when using code written by others. Therefore, we use keyword arguments. If you use keyword arguments, the values are passed from the name to the correct argument, even if they are in the wrong order.

```erg
f u: 6, v: 5, w: 4, x: 1, y: 2, z: 3
```

Note that keyword arguments and a new line immediately after the `:` are considered a colon-call style.

```erg
# means `f(x: y)`
f x: y

# means `f(x, y)`
f x:
    y
```

## Default parameters

Default parameters are used when some parameters are mostly fixed and you want to be able to omit them.

Default parameters are specified by `|=`(or-assign operator). If `base` is not specified, assign `math.E` to `base`.

```erg
math_log x: Ratio, base |= math.E = ...

assert math_log(100, 10) == 2
assert math_log(100) == math_log(100, math.E)
```

Note that there is a distinction between specifying no argument and assigning `None`.

```erg
p! x |= 0 = print!
p!(2) # 2
p!() # 0
p!(None) # None
```

Can also be used with type specification and patterns.

```erg
math_log x, base: Ratio |= math.E = ...
f [x, y] |= [1, 2] = ...
```

However, within the default arguments, it is not possible to call the procedures (described later) or assign mutable objects.

```erg
f x |= p! 1 = ... # NG
```

Also, the argument just defined cannot be used as the value passed to the default argument.

```erg
f x |= 1, y |= x = ... # NG
```

## Variable-length arguments

The `log` function, which outputs a log (record) of its arguments, can take any number of arguments.

```erg
log "Hello", "World", "!" # Hello World !
```

To define such a function, add `...` to a parameter. This way, the function receives arguments as a variable-length array.

```erg
f ...x =
    for x, i ->
        log i

# x == [1, 2, 3, 4, 5]
f 1, 2, 3, 4, 5
```

## Function definition with multiple patterns

```erg
fib n: Nat =
    match n:
        0 -> 0
        1 -> 1
        n -> fib(n - 1) + fib(n - 2)
```

Functions like the one above, where `match` appears directly under the definition, can be rewritten as follows.

```erg
fib 0 = 0
fib 1 = 1
fib(n: Nat): Nat = fib(n - 1) + fib(n - 2)
```

Note that a function definition with multiple patterns is not so-called overloading (multiple definition); a function has only a single definition. In the example above, `n` must be of the same type as `0` or `1`. Also, as with `match`, pattern matching is done from top to bottom.

If instances of different classes are mixed, the last definition must specify that the function argument is of type `Or`.

```erg
f "aa" = ...
f 1 = ...
# `f x = ... ` is invalid
f x: Int or Str = ...
```

Also, like `match`, it must also be exhaustive.

```erg
fib 0 = 0
fib 1 = 1
# PatternError: pattern of fib's parameter is not exhaustive
```

However, it can be made exhaustive by explicitly specifying the type using the [refinement type](./type/12_refinement.md) described later.

```erg
fib: 0..1 -> 0..1
fib 0 = 0
fib 1 = 1
# OK
```

## Recursive functions

A recursive function is a function that includes itself in its definition.

As a simple example, let us define a function `factorial` that performs a factorial calculation. Factorial is a computation that "multiplies all positive numbers less than or equal to".
The factorial of 5 is `5*4*3*2*1 == 120`.

```erg
factorial 0 = 1
factorial 1 = 1
factorial(n: Nat): Nat = n * factorial(n - 1)
```

First, from the definition of factorial, the factorial of 0 and 1 are both 1.
In turn, the factorial of 2 is `2*1 == 2`, the factorial of 3 is `3*2*1 == 6`, and the factorial of 4 is `4*3*2*1 == 24`.
If we look closely, we can see that the factorial of a number n is the factorial of the preceding number n-1 multiplied by n.
Putting this into code, we get `n * factorial(n - 1)`.
Since the definition of `factorial` contains itself, `factorial` is a recursive function.

As a reminder, if you do not add a type specification, it is inferred like this.

```erg
factorial: |T <: Sub(Int, T) and Mul(Int, Int) and Eq(Int)| T -> Int
factorial 0 = 1
factorial 1 = 1
factorial n = n * factorial(n - 1)
```

However, even if you can reason about it, you should explicitly specify the type of the recursive function. In the example above, a code like ``factorial(-1)`` would work, but

```erg
factorial(-1) == -1 * factorial(-2) == -1 * -2 * factorial(-3) == ...
```

and this computation does not stop. Recursive functions must carefully define the range of values or you may end up in an infinite loop.
So the type specification also helps to avoid accepting unexpected values.

## Compile-time functions

A function name begins with an uppercase letter to indicate a compile-time function. User-defined compile-time functions must have all arguments as constants and must specify their types.
Compile-time functions are limited in what they can do. Only constant expressions can be used in compile-time functions, i.e., only some operators (such as quadrature, comparison, and type construction operations) and compile-time functions. Arguments to be passed must also be constant expressions.
In return, the advantage is that the computation can be done at compile time.

```erg
Add(X, Y: Nat): Nat = X + Y
assert Add(1, 2) == 3

Factorial 0 = 1
Factorial(X: Nat): Nat = X * Factorial(X - 1)
assert Factorial(10) == 3628800

math = import "math"
Sin X = math.sin X # ConstantError: this function is not computable at compile time
```

Compile-time functions are also used in polymorphic type definitions.

```erg
Option T: Type = T or NoneType
Option: Type -> Type
```

## Appendix: Function Comparison

Erg does not define `==` for functions. This is because there is no structural equivalence algorithm for functions in general.

```erg
f = x: Int -> (x + 1)**2
g = x: Int -> x**2 + 2x + 1

assert f == g # TypeError: cannot compare functions
```

Although `f` and `g` always return the same result, it is extremely difficult to make that determination. We have to teach algebra to the compiler.
So Erg gives up on function comparisons entirely, and `(x -> x) == (x -> x)` also results in a compile error. This is a different specification from Python and should be noted.

```python
# Python, weird example
f = lambda x: x
assert f == f
assert (lambda x: x) ! = (lambda x: x)
```

## Appendix2: ()-completion

```erg
f x: Object = ...
# will be completed to
f(x: Object) = ...

f a
# will be completed to
f(a)

f a, b # TypeError: f() takes 1 positional argument but 2 were given
f(a, b) # TypeError: f() takes 1 positional argument but 2 were given
f((a, b)) # OK
```

The function type `T -> U` is actually the syntax sugar of `(T,) -> U`.

<p align='center'>
    <a href='./03_declaration.md'>Previous</a> | <a href='./05_builtin_funcs.md'>Next</a>
</p>
