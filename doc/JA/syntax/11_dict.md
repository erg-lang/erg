# 辞書

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/11_dict.md%26commit_hash%3De598201a939e24a41d3c26a828fdee01ad18eaf8)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/11_dict.md&commit_hash=e598201a939e24a41d3c26a828fdee01ad18eaf8)

Dict(辞書)はキーと値のペアを持つコレクションです。

```python
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

キーはHashable(Hashトレイトを実装した型)であるならば文字列でなくても構いません。

```python
# rangeオブジェクトをキーにするのは非推奨(スライスと混同される)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
l = {0.0: "a", 1.0: "b"} # TypeError: Float is not Hashable
```

Dictに順番は関係ありません。また、重複する要素を持つことも出来ません。この点でDictは[Set](./15_set.md)と似ています。
Dictは値付きのSetと言うこともできるでしょう。

```python,compile_fail
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

DictリテラルからDictを生成する場合、キーの重複がないかチェックされます。
重複がある場合コンパイルエラーとなりますが、自明でない場合もあり、その場合は後に登録された方が残ります(左から順番に登録されます)。

```python,compile_fail
{"Alice": 145, "Alice": 1} # KeyError: Duplicate key "Alice"
x = f(...) # x == 2
{2x+2: 1, 2(x+1): 2} # {6: 2}
```

空のDictは`{:}`で生成します。`{}`は空の配列を表すことに注意してください。

```python
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## 非等質な辞書

キー・値の型は単一でなくてもよく、そのような辞書を __非等質な辞書(heterogenous dict)__ といいます。

```python
d: {Str: Int, Int: Str} = {"a": 1, 1: "a"}
assert d["a"] == 1
assert d[1] == "a"
```

しかし、違う型のキーに同じ型の値、または同じ型のキーに違う型の値をあてることはできません。
このような場合は代わりにOr型(Union)を使います。

```python
invalid1 = {1: "a", "a": "b"}
invalid2 = {1: "a", 2: 2}

# Ergの型推論はOr型を推論しないため、型指定が必要となる
valid1: {Int or Str: Str} = {1: "a", "a": "b"}
valid2: {Int: Int or Str} = {1: "a", 2: 2}
```

## 型表示との併用

`{}`の中での`x: y`という形式は、辞書のキーと値のペアとして優先的に解釈されます。
型表示として使いたい場合は、`()`で囲む必要があります。

```python
x = "a"
{(x: Str): 1}
```

<p align='center'>
    <a href='./11_tuple.md'>Previous</a> | <a href='./13_record.md'>Next</a>
</p>
