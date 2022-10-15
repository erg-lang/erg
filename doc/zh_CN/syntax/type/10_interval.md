# 间隔类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/10_interval.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/10_interval.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

`Range` 对象最基本的用途是作为迭代器

```python
for! 0..9, i =>
    print! i
```

请注意，与 Python 不同，它包含一个结束编号

然而，这不仅仅用于 `Range` 对象。也可以使用类型。这种类型称为Interval类型

```python
i: 0..10 = 2
```

`Nat` 类型等价于 `0..<Inf` 并且，`Int` 和 `Ratio` 等价于 `-Inf<..<Inf`，
`0..<Inf` 也可以写成 `0.._`。`_` 表示任何 `Int` 类型的实例

由于它也可以用作迭代器，所以可以倒序指定，例如`10..0`，但是`<..`、`..<`和`<..<`不能倒序

```python
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # 语法错误
e = 10..<0 # 语法错误
f = 10<..<0 # 语法错误
```

Range 运算符可用于非数字类型，只要它们是"Ord"不可变类型

```python
Alphabet = "A".."z"
```

<p align='center'>
    <a href='./09_attributive.md'>上一页</a> | <a href='./11_enum.md'>下一页</a>
</p>