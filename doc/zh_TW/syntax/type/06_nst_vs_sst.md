# 記名的部分型 vs.構造的部分型


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

## 到底用 NST 還是 SST 好呢？

如果無法確定該選項，建議使用 NST。 SST 要求具備編寫不破壞任何用例的代碼的抽象能力。最好的抽象可以提高工作效率，但錯誤的抽象（__ 外觀通用性 __）會適得其反。 NST 可以降低這種風險，而不是抽象性。如果你不是庫的實現者，你可以只在 NST 中編碼。

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_inheritance.md'>Next</a>
</p>