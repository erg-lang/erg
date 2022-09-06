# IntRange L, R

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/IntRange.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/IntRange.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

`L..R`のクラス。

```python
IntRange L, R: Int == L..R
```

## 方法

* .`_+_`: Self(L1, R1), Self(L2, R2) -> Self(L1+L2, R1+R2)

正常加法。 `Int` 和 `Nat` 的添加在此定義為假裝它在每個類中定義

```python
0..10 + 1..12 == 1..22
Int + 0..10 == _..|Int|_ + 0..10 == _..|Int|_ == Int
Nat + Nat == 0.._ + 0.._ == 0.._ == Nat
```
