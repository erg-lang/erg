# 間隔類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/10_interval.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/10_interval.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

`Range` 對象最基本的用途是作為迭代器

```python
for! 0..9, i =>
    print! i
```

請注意，與 Python 不同，它包含一個結束編號

然而，這不僅僅用于 `Range` 對象。 也可以使用類型。 這種類型稱為Interval類型

```python
i: 0..10 = 2
```

`Nat` 類型等價于 `0..<Inf` 并且，`Int` 和 `Ratio` 等價于 `-Inf<..<Inf`，
`0..<Inf` 也可以寫成 `0.._`。 `_` 表示任何 `Int` 類型的實例

由于它也可以用作迭代器，所以可以倒序指定，例如`10..0`，但是`<..`、`..<`和`<..<`不能倒序

```python
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # 語法錯誤
e = 10..<0 # 語法錯誤
f = 10<..<0 # 語法錯誤
```

Range 運算符可用于非數字類型，只要它們是"Ord"不可變類型

```python
Alphabet = "A".."z"
```
