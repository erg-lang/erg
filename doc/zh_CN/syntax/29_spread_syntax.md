# 扩展语法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_spread_syntax.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_spread_syntax.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

在分解赋值中，将 `*` 放在变量前面会将所有剩余元素展开到该变量中。这称为展开赋值

```python
[x, *y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, *y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## 提取赋值

提取分配是一种方便的语法，用于本地化模块或记录中的特定属性

```python
{sin; cos; tan} = import "math"
```

之后，您可以在本地使用`sin，cos，tan`

您可以对记录执行相同的操作。

```python
record = {x = 1; y = 2}
{x; y} = record
```

如果要全部展开，请使用`{*}=record`。它在OCaml中是`open`。

```python
record = {x = 1; y = 2}
{*} = records
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./28_comprehension.md'>上一页</a> | <a href='./30_decorator.md'>下一页</a>
</p>
