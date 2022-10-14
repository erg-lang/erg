# 名义子类型与结构子类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/06_nst_vs_sst.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/06_nst_vs_sst.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

```python
Months = 0..12

# NST
MonthsClass = Class Months
MonthsClass.
    name self =
        match self:
            1 -> "january"
            2 -> "february"
            3 -> "march"
            ...

# SST
MonthsImpl = Patch Months
MonthsImpl.
    name self =
        match self:
            1 -> "January"
            2 -> "February"
            3 -> "March"
            ...

assert 12 in Months
assert 2.name() == "February"
assert not 12 in MonthsClass
assert MonthsClass.new(12) in MonthsClass
# 它可以使用结构类型，即使包装在一个类中
assert MonthsClass.new(12) in Months
# 如果两者都存在，则类方法优先
assert MonthsClass.new(2).name() == "february"
```

## 最后，我应该使用哪个，NST 还是 SST?

如果您无法决定使用哪一个，我们的建议是 NST
SST 需要抽象技能来编写在任何用例中都不会崩溃的代码。好的抽象可以带来高生产力，但错误的抽象(外观上的共性)会导致适得其反的结果。(NST 可以通过故意将抽象保持在最低限度来降低这种风险。如果您不是库实现者，那么仅使用 NST 进行编码并不是一个坏主意

<p align='center'>
    <a href='./04_class.md'>上一页</a> | <a href='./06_inheritance.md'>下一页</a>
</p>
