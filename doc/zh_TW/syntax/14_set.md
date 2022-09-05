# Set

一個Set代表一個集合，它在結構上是一個重復的無序數組。

```python
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # 重復的被自動刪除
assert {1, 2} == {2, 1}
```

Set可以執行集合操作。

```python
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

Set是同質集合。 為了使不同類的對象共存，它們必須同質化

```python
s: {Int or Str} = {"a", 1, "b", -1}
```

## Sets為類型
Sets也可以被視為類型。 這種類型稱為 _枚舉類型_。

```python
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

Set的元素直接是類型的元素。
請注意，這些Set本身是不同的。

```python
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>上一頁</a> | <a href='./15_type.md'>下一頁</a>
</p>