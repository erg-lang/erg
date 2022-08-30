# Dict

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/12_dict.md%26commit_hash%3D7a2f59795bc983f2fd0daf9732f32d59bd8e9c59)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/12_dict.md&commit_hash=7a2f59795bc983f2fd0daf9732f32d59bd8e9c59)

Dict is a collection of key/value pairs.

```erg
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

The key does not have to be a string if it is a `Hash` object.

```erg
# deprecated to use a range object as a key (confused with slice)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
```

Order does not matter for Dict. It also cannot have duplicate elements. In this respect, Dict is similar to Set.
You could say that a Dict is a Set with values.

```erg
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

When generating a dict from a dict literal, it is checked for duplicate keys.
Any duplicates will result in a compile error.

```erg
{"Alice": 145, "Alice": 1} # KeyError: Duplicate key "Alice"
```

Empty Dict is created with `{:}`. Note that `{}` denotes an empty set.

```erg
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## Heterogeneous Dict

There need not be a single key/value type. Such a dictionary is called a __heterogenous dict_.

```erg
d: {Str: Int, Int: Str} = {"a": 1, 1: "a"}
assert d["a"] == 1
assert d[1] == "a"
```

However, it is not possible to assign values of the same type to keys of different types, or values of different types to keys of the same type.
In such cases, use the type Or instead.

```erg
invalid1 = {1: "a", "a": "b"}
invalid2 = {1: "a", 2: 2}

# Erg type inference does not infer Or type, so type specification is required
valid1: {Int or Str: Str} = {1: "a", "a": "b"}
valid2: {Int: Int or Str} = {1: "a", 2: 2}
```

<p align='center'>
    <a href='./11_tuple.md'>Previous</a> | <a href='./13_record.md'>Next</a>
</p>
