# 字典

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/11_dict.md%26commit_hash%3Dd737ab144180f6c4dcce7685d81c67afa6a1f387)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/11_dict.md&commit_hash=d737ab144180f6c4dcce7685d81c67afa6a1f387)


Dict 是键/值对的集合

```python
ids = {"Alice": 145, "Bob": 214, "Charlie": 301}
assert ids["Alice"] == 145
```

如果键是"哈希"对象，则键不必是字符串

```python
# 不推荐使用范围对象作为键(与切片混淆)
r = {1..3: "1~3", 4..6: "4~6", 7..9: "7~9"}
assert r[1..3] == "1~3"
l = {[]: "empty", [1]: "1"}
assert l[[]] == "empty"
```

对于字典来说，顺序无关紧要。它也不能有重复的元素。在这方面，Dict 与 Set 类似
您可以说 Dict 是具有值的 Set

```python
{"Alice": 145, "Bob": 214, "Charlie": 301} == {"Alice": 145, "Charlie": 301, "Bob": 214}
```

从 dict 文字生成 dict 时，会检查重复键
任何重复都会导致编译错误

```python,compile_fail
{"Alice": 145, "Alice": 1} # Key错误: 重复键`Alice`
```

空字典是用 `{:}` 创建的。请注意，`{}` 表示一个空集

```python
mut_dict = !{:}
mut_dict.insert! "Alice", 145
mut_dict.insert! "Bob", 214
assert mut_dict["Alice"] == 145
```

## 异构字典

不需要有单一的键/值类型。这样的字典称为 __heterogenous dict_

```python
d: {Str: Int, Int: Str} = {"a": 1, 1: "a"}
assert d["a"] == 1
assert d[1] == "a"
```

但是，不能将相同类型的值分配给不同类型的键，或者将不同类型的值分配给相同类型的键
在这种情况下，请改用 Or 类型

```python
invalid1 = {1: "a", "a": "b"}
invalid2 = {1: "a", 2: 2}

# Erg 类型推断不推断 Or 类型，因此需要类型说明
valid1: {Int or Str: Str} = {1: "a", "a": "b"}
valid2: {Int: Int or Str} = {1: "a", 2: 2}
```

<p align='center'>
    <a href='./10_array.md'>上一页</a> | <a href='./12_container_ownership.md'>下一页</a>
</p>
