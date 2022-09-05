# 傳播賦值

在分解賦值中，將 `...` 放在變量前面會將所有剩余元素展開到該變量中。 這稱為擴展賦值。

```python
[x,...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## 提取賦值

如果在 `...` 之后沒有寫入任何內容，則忽略并分配剩余的元素。 這種類型的擴展賦值具體稱為抽取賦值。
提取分配是一種方便的語法，用于本地化模塊或記錄中的特定屬性。

```python
{sin; cos; tan; ..} = import "math"
```

After that, you can use `sin, cos, tan` locally.

You can do the same with records.

```python
record = {x = 1; y = 2}
{x; y; ...} = record
```

If you want to expand all, use `{*} = record`. It is `open` in OCaml.

```python
record = {x = 1; y = 2}
{*} = records
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>上一頁</a> | <a href='./29_decorator.md'>下一頁</a>
</p>