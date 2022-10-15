# Type Widening

For example, define the polycorrelation coefficient as follows.

```python
ids|T|(x: T, y: T) = x, y
```

There's nothing wrong with assigning a pair of instances of the same class.
When you assign an instance pair of another class that has a containment relationship, it is upcast to the larger one and becomes the same type.
Also, it is easy to understand that an error will occur if another class that is not in the containment relationship is assigned.

```python
assert ids(1, 2) == (1, 2)
assert ids(1, 2.0) == (1.0, 2.0)
ids(1, "a") # TypeError
```

Now, what about types that have different derived types?

```python
i: Int or Str
j: Int or NoneType
ids(i, j) # ?
```

Before explaining this, we have to focus on the fact that Erg's type system doesn't actually look at (runtime) classes.

```python
1: {__valueclass_tag__ = Phantom Int}
2: {__valueclass_tag__ = Phantom Int}
2.0: {__valueclass_tag__ = Phantom Ratio}
"a": {__valueclass_tag__ = Phantom Str}
ids(1, 2): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Int} == {__valueclass_tag__ = Phantom Int}
ids(1, 2.0): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Ratio} == {__valueclass_tag__ = Phantom Ratio} # Int < Ratio
ids(1, "a"): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Str} == Never # TypeError
```

I don't see the class because it may not be seen exactly, because in Erg the class of an object belongs to runtime information.
For example, the class of an `Int or Str` type object is either `Int` or `Str`, but you can only know which one it is by executing it.
Of course, the class of an object of type `Int` is defined as `Int`, but in this case as well, what is visible from the type system is the structural type `{__valueclass_tag__ = Int}` of `Int`.

Now let's go back to another structured type example. In conclusion, the above code will result in a TypeError as the type does not match.
However, if you do type expansion with type annotations, compilation will pass.

```python
i: Int or Str
j: Int or NoneType
ids(i, j) # TypeError: types of i and j not matched
# hint: try type widening (e.g. ids<Int or Str or NoneType>)
ids<Int or Str or NoneType>(i, j) # OK
```

`A and B` have the following possibilities.

* `A and B == A`: when `A <: B` or `A == B`.
* `A and B == B`: when `A :> B` or `A == B`.
* `A and B == {}`: when `!(A :> B)` and `!(A <: B)`.

`A or B` has the following possibilities.

* `A or B == A`: when `A :> B` or `A == B`.
* `A or B == B`: when `A <: B` or `A == B`.
* `A or B` is irreducible (independent types): if `!(A :> B)` and `!(A <: B)`.

## Type widening in subroutine definitions

Erg defaults to an error if return types do not match.

```python
parse_to_int s: Str =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # return Int object
# TypeError: mismatched types of return values
# 3 | do parse_to_int::return error("not numeric")
# └─ Error
# 4 | ...
# └ Int
```

In order to solve this, it is necessary to explicitly specify the return type as Or type.

```python
parse_to_int(s: Str): Int or Error =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # return Int object
```

This is by design so that you don't unintentionally mix a subroutine's return type with another type.
However, if the return value type option is a type with an inclusion relationship such as `Int` or `Nat`, it will be aligned to the larger one.