# Interval begin, end := WellOrder

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Interval.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Interval.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

表示有序集合類型 (WellOrder) 的子類型的類型。Interval 類型具有派生類型，例如 PreOpen(x<..y)。

```python
Months = 1..12
Alphabet = "a".."z"
Weekdays = Monday..Friday
Winter = November..December or January..February
```

```python
0..1 # 整數范圍
0.0..1.0 # 真實(有理)范圍
# 或 0/1..1/1 相同
```

計算機無法處理無限位數的數字，所以實數的范圍實際上是有理數的范圍。