# Interval begin, end := WellOrder

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Interval.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Interval.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

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
