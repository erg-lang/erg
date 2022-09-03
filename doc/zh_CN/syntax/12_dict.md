# Dict

Dict 是一个包含键值对的集合。


```erg
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

如果密钥为 Hash，则该密钥可以不是字符串。


```erg
# rangeオブジェクトをキーにするのは非推奨(スライスと混同される)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
```

顺序对迪奇并不重要。也不能有重复的元素。在这一点上，Dict 与相似。Dict 也可以说是一个有价值的 Set。


```erg
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

从 Dict 文字生成 Dict 时，将检查是否存在重复的键。如果存在重复项，则会导致编译错误。


```erg
{"Alice": 145, "Alice": 1} # KeyError: Duplicate key "Alice"
```

使用生成空 Dict。请注意，表示空数组。


```erg
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## Heterogeneous Dict

键值的类型可以不是单一的，这样的字典称为。


```erg
d: {Str: Int, Int: Str} = {”a”: 1, 1: “a”}
assert d[”a”] == 1
assert d[1] == “a”
```

但是，不能将相同类型的值应用于不同类型的键，也不能将不同类型的值应用于不同类型的键。在这些情况下，请改用 Or 类型（Union）。


```erg
invalid1 = {1: “a”, “a”: “b”}
invalid2 = {1: “a”, 2: 2}

# Ergの型推論はOr型を推論しないので、型指定が必要
valid1: {Int or Str: Str} = {1: “a”, “a”: “b”}
valid2: {Int: Int or Str} = {1: “a”, 2: 2}
```

<p align='center'>
    <a href='./11_tuple.md'>Previous</a> | <a href='./13_record.md'>Next</a>
</p>
