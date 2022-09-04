# Dict

Dict 是一個包含鍵值對的集合。


```erg
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

如果密鑰為 Hash，則該密鑰可以不是字符串。


```erg
# rangeオブジェクトをキーにするのは非推奨(スライスと混同される)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
```

順序對迪奇並不重要。也不能有重複的元素。在這一點上，Dict 與相似。 Dict 也可以說是一個有價值的 Set。


```erg
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

從 Dict 文字生成 Dict 時，將檢查是否存在重複的鍵。如果存在重複項，則會導致編譯錯誤。


```erg
{"Alice": 145, "Alice": 1} # KeyError: Duplicate key "Alice"
```

使用生成空 Dict。請注意，表示空數組。


```erg
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## Heterogeneous Dict

鍵值的類型可以不是單一的，這樣的字典稱為。


```erg
d: {Str: Int, Int: Str} = {”a”: 1, 1: “a”}
assert d[”a”] == 1
assert d[1] == “a”
```

但是，不能將相同類型的值應用於不同類型的鍵，也不能將不同類型的值應用於不同類型的鍵。在這些情況下，請改用 Or 類型（Union）。


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