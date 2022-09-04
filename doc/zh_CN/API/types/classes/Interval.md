# Interval begin, end := WellOrder

表示有序集合类型 (WellOrder) 的子类型的类型。Interval 类型具有派生类型，例如 PreOpen(x<..y)。

```python
Months = 1..12
Alphabet = "a".."z"
Weekdays = Monday..Friday
Winter = November..December or January..February
```

```python
0..1 # 整数范围
0.0..1.0 # 真实（有理）范围
# 或 0/1..1/1 相同
```

计算机无法处理无限位数的数字，所以实数的范围实际上是有理数的范围。