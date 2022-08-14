# Dict

Dictはキーと値のペアを持つコレクションです。

```erg
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

キーはHashであるならば文字列でなくても構いません。

```erg
# rangeオブジェクトをキーにするのは非推奨(スライスと混同される)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
```

Dictに順番は関係ありません。また、重複する要素を持つことも出来ません。この点でDictは[Set](./14_set.md)と似ています。
Dictは値付きのSetと言うこともできるでしょう。

```erg
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

DictリテラルからDictを生成する場合、キーの重複がないかチェックされます。
重複がある場合コンパイルエラーとなります。

```erg
{"Alice": 145, "Alice": 1} # KeyError: Duplicate key "Alice"
```

空のDictは`{:}`で生成します。`{}`は空の配列を表すことに注意してください。

```erg
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## Heterogeneous Dict

キー・値の型は単一でなくてもよく、そのような辞書を __非等質な辞書(heterogenous dict)__ といいます。

```erg
d: {Str: Int, Int: Str} = {”a”: 1, 1: “a”}
assert d[”a”] == 1
assert d[1] == “a”
```

しかし、違う型のキーに同じ型の値、または同じ型のキーに違う型の値をあてることはできません。
このような場合は代わりにOr型(Union)を使います。

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
