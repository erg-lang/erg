# 记名的部分型 vs.构造的部分型


```erg
Months = 0..12

# NST
MonthsClass = Class Months
MonthsClass.
    name self =
        match self:
            1 -> "January"
            2 -> "February"
            3 -> "March"
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
# クラスでラップしても構造型は使える
assert MonthsClass.new(12) in Months
# 両方ある場合クラスメソッドが優先
assert MonthsClass.new(2).name() == "february"
```

## 到底用 NST 还是 SST 好呢？

如果无法确定该选项，建议使用 NST。SST 要求具备编写不破坏任何用例的代码的抽象能力。最好的抽象可以提高工作效率，但错误的抽象（__ 外观通用性 __）会适得其反。NST 可以降低这种风险，而不是抽象性。如果你不是库的实现者，你可以只在 NST 中编码。

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_inheritance.md'>Next</a>
</p>
