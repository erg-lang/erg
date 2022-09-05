# IntRange L, R

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
