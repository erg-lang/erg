# Array

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/10_array.md%26commit_hash%3D6c6afe84d1dc05ee7566b46c12d39b8c49a3acfb)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/10_array.md&commit_hash=6c6afe84d1dc05ee7566b46c12d39b8c49a3acfb)

Arrays are the most basic __collection (aggregate)__.
A collection is an object that can hold multiple objects inside it.

```erg
a = [1, 2, 3]
a: [Int; 3] # Type specification: number after semicolon is the number of elements
# Can be omitted if the number of elements is not known
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

As a rule, arrays cannot contain objects of different types.

```erg.
[1, "a"] # TypeError: 1st element is Int, but 2nd element is Str
```

However, you can bypass the restriction by explicitly specifying the type like this.

```erg
[1, "a"]: [Int or Str].
```

## Slice

An array can also have multiple values taken out at once. This is called slicing.

```erg
l = [1, 2, 3, 4]
# Same as l[1:3] in Python
assert l[1.. <3] == [2, 3]
assert l[1..2] == [2, 3]
# Same as l[1]
assert l[1..1] == [2]
# Same as l[::2] in Python
assert l[..].step(2) == [2, 4]
```

The object obtained by slicing is an (immutable) copy to an array.

```erg
print! Typeof l[1..2] # [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>Previous</a> | <a href='./11_tuple.md'>Next</a>
</p>
