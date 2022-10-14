# 传播赋值

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/28_spread_syntax.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/28_spread_syntax.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

在分解赋值中，将 `...` 放在变量前面会将所有剩余元素展开到该变量中。这称为扩展赋值

```python
[x,...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## 提取赋值

如果在 `...` 之后没有写入任何内容，则忽略并分配剩余的元素。这种类型的扩展赋值具体称为抽取赋值
提取分配是一种方便的语法，用于本地化模块或记录中的特定属性

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
    <a href='./27_comprehension.md'>上一页</a> | <a href='./29_decorator.md'>下一页</a>
</p>