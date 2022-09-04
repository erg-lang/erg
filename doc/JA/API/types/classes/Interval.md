# Interval begin, end := WellOrder

整列集合型(WellOrder)の部分型を表す型です。Interval型にはPreOpen(x<..y)などの派生型が存在します。

```python
Months = 1..12
Alphabet = "a".."z"
Weekdays = Monday..Friday
Winter = November..December or January..February
```

```python
0..1 # 整数の範囲
0.0..1.0 # 実数(有理数)の範囲
# or 0/1..1/1でも同じ
```

コンピュータは無限桁の数を上手く扱えないので、実数の範囲と言っても実際は有理数の範囲である。
