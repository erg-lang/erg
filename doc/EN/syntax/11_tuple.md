# Tuple

Tuples are similar to arrays, but can hold objects of different types.
Such a collection is called an unequal collection. In contrast, homogeneous collections include arrays, sets, etc.

```python
t = (1, True, "a")
(i, b, s) = t
assert(i == 1 and b == True and s == "a")
```

The tuple `t` can retrieve the nth element in the form `t.n`; note that unlike Python, it is not `t[n]`.
This is because accessing tuple elements is more like an attribute (the existence of the element is checked at compile time, and the type can change depending on `n`) than a method (an array's `[]` is a method).

```python
assert t.0 == 1
assert t.1 == True
assert t.2 == "a"
```

Parentheses `()` are optional when not nested.

```python
t = 1, True, "a"
i, b, s = t
```

Tuples can hold objects of different types, so they cannot be iterated like arrays.

```python
t: ({1}, {2}, {3}) = (1, 2, 3)
(1, 2, 3).iter().map(x -> x + 1) # TypeError: type ({1}, {2}, {3}) has no method `.iter()`
# If all types are the same, they can be represented by `(T; n)` like arrays, but this still does not allow iteration
t: (Int; 3) = (1, 2, 3)
assert (Int; 3) == (Int, Int, Int)
```

However, nonhomogeneous collections (such as tuples) can be converted to homogeneous collections (such as arrays) by upcasting, intersecting, and so on.
This is called equalization.

```python
(Int, Bool, Str) can be [T; 3] where T :> Int, T :> Bool, T :> Str
```

```python
t: (Int, Bool, Str) = (1, True, "a") # non-homogenous
a: [Int or Bool or Str; 3] = [1, True, "a"] # homogenous
_a: [Show; 3] = [1, True, "a"] # homogenous
_a.iter().map(x -> log x) # OK
t.try_into([Show; 3])? .iter().map(x -> log x) # OK
```

## Unit

A tuple with zero elements is called a __unit__. A unit is a value, but also refers to its own type.

```python
unit = ()
(): ()
```

Unit is a superclass of all element 0 tuples.

```python
() > (Int; 0)
() > (Str; 0)
```

The use of this object is for procedures with no arguments and no return value, etc. Erg subroutines must have arguments and a return value. However, in some cases, such as a procedure, there may be no meaningful arguments or return value, only side effects. In such cases, we use units as "meaningless, formal values.

```python
# ↓ Actually, this parenthesis is a unit
p!() =.
    # `print!` does not return a meaningful value
    print! "Hello, world!"
p!: () => ()
```

However, Python tends to use `None` instead of units in such cases.
In Erg, you should use `()` when you are sure from the beginning that the operation will not return a meaningful value, such as in a procedure, and return `None` when there is a possibility that the operation will fail and you will get nothing, such as when retrieving an element.

## Arguments and Tuple

Actually, all of Erg's `Callable` objects are one argument and one return value; a subroutine that takes N arguments was just receiving "one tuple with N elements" as an argument.

```python
# f x = ... is implicitly assumed to be f(x) = ... is considered to be
f x = x
assert f(1) == 1
f(1, 2, 3) # ArgumentError: f takes 1 positional argument but 3 were given
# ArgumentError: f takes 1 positional argument but 3 were given
g x: Int, . . y: Int = y
assert (2, 3) == g 1, 2, 3
```

This also explains the function type.

```python
assert f in T: {(T,) -> T | T}
assert g in {(Int, ... (Int; N)) -> (Int; N) | N: Nat}
```

To be precise, the function's input is not a tuple but a "Named tuple with default attributes". This is a special tuple that can only be used in function arguments, can be named like a record, and can have a default value.

```python
f(x: Int, y=0) = x + y
f: (Int, y=Int) -> Int

f(x=0, y=1)
f(y=1, x=0)
f(x=0)
f(0)
```

<p align='center'>
    <a href='./10_array.md'>Previous</a> | <a href='./12_dict.md'>Next</a>
</p>
