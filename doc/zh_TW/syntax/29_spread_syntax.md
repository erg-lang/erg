# 擴展語法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_spread_syntax.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_spread_syntax.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

在分解賦值中，將 `*` 放在變量前面會將所有剩余元素展開到該變量中。這稱為展開賦值

```python
[x, *y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, *y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## 提取賦值

提取分配是一種方便的語法，用于本地化模塊或記錄中的特定屬性

```python
{sin; cos; tan} = import "math"
```

之后，您可以在本地使用`sin，cos，tan`

您可以對記錄執行相同的操作。

```python
record = {x = 1; y = 2}
{x; y} = record
```

如果要全部展開，請使用`{*}=record`。它在OCaml中是`open`。

```python
record = {x = 1; y = 2}
{*} = records
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./28_comprehension.md'>上一頁</a> | <a href='./30_decorator.md'>下一頁</a>
</p>
