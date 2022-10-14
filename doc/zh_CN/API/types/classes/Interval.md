# Interval begin, end := WellOrder

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Interval.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Interval.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

表示有序集合类型 (WellOrder) 的子类型的类型。Interval 类型具有派生类型，例如 PreOpen(x<..y)

```python
Months = 1..12
Alphabet = "a".."z"
Weekdays = Monday..Friday
Winter = November..December or January..February
```

```python
0..1 # 整数范围
0.0..1.0 # 真实(有理)范围
# 或 0/1..1/1 相同
```

计算机无法处理无限位数的数字，所以实数的范围实际上是有理数的范围。