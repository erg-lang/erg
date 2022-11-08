# Special form

Special forms are operators, subroutines (and the like) that cannot be expressed in the Erg type system. It is surrounded by ``, but it cannot actually be captured.
Also, types such as `Pattern`, `Body`, and `Conv` appear for convenience, but such types do not exist. Its meaning also depends on the context.

## `=`(pat: Pattern, body: Body) -> NoneType

Assign body to pat as a variable. Raise an error if the variable already exists in the same scope or if it doesn't match pat.
It is also used in record attribute definitions and default arguments.

```python
record = {i = 1; j = 2}
f(x: Int, y = 2) = ...
```

`=` has special behavior when the body is a type or a function.
The variable name on the left side is embedded in the object on the right side.

```python
print! Class() # <class <lambda>>
print! x: Int -> x + 1 # <function <lambda>>
C = Class()
print! c # <class C>
f = x: Int -> x + 1
print! f # <function f>
gx: Int = x + 1
print! g # <function g>
KX: Int = Class(...)
print! K # <kind K>
L = X: Int -> Class(...)
print! L # <kind L>
```

The `=` operator has a return value of "undefined".
Multiple assignments and `=` in functions result in syntax errors.

```python
i = j = 1 # SyntaxError: multiple assignments are not allowed
print!(x=1) # SyntaxError: cannot use `=` in function arguments
# hint: did you mean keyword arguments (`x: 1`)?
if True, do:
    i = 0 # SyntaxError: A block cannot be terminated by an assignment expression
```

## `->`(pat: Pattern, body: Body) -> Func

Generate anonymous functions, function types.

## `=>`(pat: Pattern, body: Body) -> Proc

Generate anonymous procedure, procedure type.

## `:`(subject, T)

Determine if subject matches T. If they don't match, throw a compile error.

```python
a: Int
f x: Int, y: Int = x / y
```

Also used for `:` applied styles.

```python
fx:
    y
    z
```

Like `:` and `=`, the result of the operation is undefined.

```python
_ = x: Int # SyntaxError:
print!(x: Int) # SyntaxError:
```

## `.`(obj, attr)

Read attributes of obj.
`x.[y, z]` will return the y and z attributes of x as an array.

## `|>`(obj, c: Callable)

Execute `c(obj)`. `x + y |>.foo()` is the same as `(x + y).foo()`.

### |T: Type|(x: Option T)`?` -> T

Postfix operator. Call `x.unwrap()` and `return` immediately in case of error.

## match(obj, ...lambdas: Lambda)

For obj, execute lambdas that match the pattern.

```python
match[1, 2, 3]:
  (l: Int) -> log "this is type of Int"
  [[a], b] -> log a, b
  [...a] -> log a
# (one two three)
```

## del|T: Type|(x: ...T) -> NoneType

Delete the variable `x`. However, built-in objects cannot be deleted.

```python
a = 1
del a # OK

del True # SyntaxError: cannot delete a built-in object
```

## do(body: Body) -> Func

Generate an anonymous function with no arguments. Syntactic sugar for `() ->`.

## do!(body: Body) -> Proc

Generate an anonymous procedure with no arguments. Syntactic sugar for `() =>`.

## `else`(l, r) -> Choice

Creates a tuple-like structure of two pairs called Choice objects.
`l, r` are evaluated lazily. That is, the expression is evaluated only when `.get_then` or `.get_else` is called.

```python
choice = 1 else 2
assert choice.get_then() == 1
assert choice.get_else() == 2
assert True.then(choice) == 1
```

## set operator

### `[]`(...objs)

Creates an array from arguments or a dict from optional arguments.

### `{}`(...objs)

Create a set from arguments.

### `{}`(...fields: ((Field, Value); N))

Generate a record.

### `{}`(layout, ...names, ...preds)

Generates refinement type, rank 2 type.

### `...`

Expand a nested collection. It can also be used for pattern matching.

```python
[x,...y] = [1, 2, 3]
assert x == 1 and y == [2, 3]
assert [x, ...y] == [1, 2, 3]
assert [...y, x] == [2, 3, 1]
{x; ...yz} = {x = 1; y = 2; z = 3}
assert x == 1 and yz == {y = 2; z = 3}
assert {x; ...yz} == {x = 1; y = 2; z = 3}
```

## virtual operator

Operators that cannot be used directly by the user.

### ref|T: Type|(x: T) -> Ref T

Returns an immutable reference to the object.

### ref!|T!: MutType|(x: T!) -> Ref! T!

Returns a mutable reference to a mutable object.
