# Interval begin, end := WellOrder

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