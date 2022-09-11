# 字典

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/12_dict.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/12_dict.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

Dict 是鍵/值對的集合。

```python
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

如果鍵是"哈希"對象，則鍵不必是字符串。

```python
# 不推薦使用范圍對象作為鍵(與切片混淆)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
```

對于字典來說，順序無關緊要。 它也不能有重復的元素。 在這方面，Dict 與 Set 類似。
您可以說 Dict 是具有值的 Set。

```python
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

從 dict 文字生成 dict 時，會檢查重復鍵。
任何重復都會導致編譯錯誤。

```python
{"Alice": 145, "Alice": 1} # Key錯誤：重復鍵`Alice`
```

空字典是用 `{:}` 創建的。 請注意，`{}` 表示一個空集。

```python
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## 異構字典

不需要有單一的鍵/值類型。 這樣的字典稱為 __heterogenous dict_。

```python
d: {Str: Int, Int: Str} = {"a": 1, 1: "a"}
assert d["a"] == 1
assert d[1] == "a"
```

但是，不能將相同類型的值分配給不同類型的鍵，或者將不同類型的值分配給相同類型的鍵。
在這種情況下，請改用 Or 類型。

```python
invalid1 = {1: "a", "a": "b"}
invalid2 = {1: "a", 2: 2}

# Erg 類型推斷不推斷 Or 類型，因此需要類型說明
valid1: {Int or Str: Str} = {1: "a", "a": "b"}
valid2: {Int: Int or Str} = {1: "a", 2: 2}
```

<p align='center'>
    <a href='./11_tuple.md'>上一頁</a> | <a href='./13_record.md'>下一頁</a>
</p>
