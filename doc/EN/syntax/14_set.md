# Set

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/14_set.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/14_set.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

A set is an unordered array with no duplicates.

```erg
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # duplicates are automatically removed
assert {1, 2} == {2, 1}
```

Sets can perform mathematical set operations.

```erg
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

A set is a homogenous collection. Objects of different classes must be made equal in order to coexist.

```erg
s1 = {"a", 1, "b", -1} # TypeError
s2: {Int or Str} = {"a", 1, "b", -1}
```

## Set as Type

Sets can also be treated as types. Such a type is called an __Enum type_.

```erg
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

The elements of the set are directly the elements of the type.
Note that the set itself is different.

```erg
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>Previous</a> | <a href='./15_type.md'>Next</a>
</p>
