# Set

A set represents a collection, which is structurally a duplicate, unordered array.

```python
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # duplicates are automatically removed
assert {1, 2} == {2, 1}
```

Sets can be declared by specifying type and length.

```python
a: {Int; 3} = {0, 1, 2} # OK
b: {Int; 3} = {0, 0, 0} # NG, Duplicates are deleted, and the length changes.
# TypeError: the type of b is mismatched
# expected:  Set(Int, 3)
# but found: Set({0, }, 1)
```

In addition, only objects that implement the `Eq` trait can be elements of the Set.

Therefore, it is not possible to use the Set elements such as a Float.

```python,compile_fail
d = {0.0, 1.0} # NG
#
# 1│ d = {0.0, 1.0}
#         ^^^^^^^^
# TypeError: the type of _ is mismatched:
# expected:  Eq(Float)
# but found: {0.0, 1.0, }
```

Sets can perform set operations.

```python
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

A set is a homogeneous collection. In order for objects of different classes to coexist, they must be homogenized.

```python
s: {Int or Str} = {"a", 1, "b", -1}
```

## Sets as types

Sets can also be treated as types. Such types are called __Enum types__.

```python
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

Elements of the set are directly elements of the type.
Note that the sets themselves are different.

```python
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>Previous</a> | <a href='./15_type.md'>Next</a>
</p>
