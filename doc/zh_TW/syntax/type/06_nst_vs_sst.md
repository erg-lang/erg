# 名義子類型與結構子類型

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
# 它可以使用結構類型，即使包裝在一個類中。
assert MonthsClass.new(12) in Months
# 如果兩者都存在，則類方法優先
assert MonthsClass.new(2).name() == "february"
```

## 最后，我應該使用哪個，NST 還是 SST？

如果您無法決定使用哪一個，我們的建議是 NST。
SST 需要抽象技能來編寫在任何用例中都不會崩潰的代碼。 好的抽象可以帶來高生產力，但錯誤的抽象(外觀上的共性)會導致適得其反的結果。(NST 可以通過故意將抽象保持在最低限度來降低這種風險。如果您不是庫實現者，那么僅使用 NST 進行編碼并不是一個壞主意。

<p align='center'>
    <a href='./04_class.md'>上一頁</a> | <a href='./06_inheritance.md'>下一頁</a>
</p>
