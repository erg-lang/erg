# Set

A set represents a collection, which is structurally a duplicate, unordered array.

``` erg
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # duplicates are automatically removed
assert {1, 2} == {2, 1}
```

Sets can perform set operations.

``` erg
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

A set is a homogeneous collection. In order for objects of different classes to coexist, they must be homogenized.

``` erg
s: {Int or Str} = {"a", 1, "b", -1}
```

## Sets as types

Sets can also be treated as types. Such types are called __Enum types__.

``` erg
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

Elements of the set are directly elements of the type.
Note that the sets themselves are different.

``` erg
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>Previous</a> | <a href='./15_type.md'>Next</a>
</p>