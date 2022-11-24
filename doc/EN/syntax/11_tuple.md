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

Unit is a superclass of all tuples.

```python
() :> (Int; 0)
() :> (Str; 0)
() :> (Int, Str)
...
```

The use of this object is for procedures with no arguments and no return value, etc. Erg subroutines must have arguments and a return value. However, in some cases, such as a procedure, there may be no meaningful arguments or return value, only side effects. In such cases, we use units as "meaningless, formal values.

```python
p!() =.
    # `print!` does not return a meaningful value
    print! "Hello, world!"
p!: () => () # The parameter part is part of the syntax, not a tuple
```

However, Python tends to use `None` instead of units in such cases.
In Erg, you should use `()` when you are sure from the beginning that the operation will not return a meaningful value, such as in a procedure, and return `None` when there is a possibility that the operation will fail and you will get nothing, such as when retrieving an element.

<p align='center'>
    <a href='./10_array.md'>Previous</a> | <a href='./12_dict.md'>Next</a>
</p>
