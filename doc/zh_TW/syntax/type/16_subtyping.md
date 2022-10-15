# 子類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/16_subtyping.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/16_subtyping.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

在 Erg 中，可以使用比較運算符 `<`、`>` 確定類包含

```python
Nat < Int
Int < Object
1... _ < Nat
{1, 2} > {1}
{=} > {x = Int}
{I: Int | I >= 1} < {I: Int | I >= 0}
```

請注意，這與 `<:` 運算符的含義不同。它聲明左側的類是右側類型的子類型，并且僅在編譯時才有意義

```python
C <: T # T: 結構類型
f|D <: E| ...

assert F < G
```

您還可以為多態子類型規范指定 `Self <: Add`，例如 `Self(R, O) <: Add(R, O)`

## 結構類型和類類型關系

結構類型是結構類型的類型，如果它們具有相同的結構，則被認為是相同的對象

```python
T = Structural {i = Int}
U = Structural {i = Int}

assert T == U
t: T = {i = 1}
assert t in T
assert t in U
```

相反，類是符號類型的類型，不能在結構上與類型和實例進行比較

```python
C = Class {i = Int}
D = Class {i = Int}

assert C == D # 類型錯誤: 無法比較類
c = C.new {i = 1}
assert c in C
assert not c in D
```

## 子程序的子類型化

子例程的參數和返回值只采用一個類
換句話說，您不能直接將結構類型或Trait指定為函數的類型
必須使用部分類型規范將其指定為"作為該類型子類型的單個類"

```python
# OK
f1 x, y: Int = x + y
# NG
f2 x, y: Add = x + y
# OK
# A 是一些具體的類
f3<A <: Add> x, y: A = x + y
```

子程序中的類型推斷也遵循此規則。當子例程中的變量具有未指定的類型時，編譯器首先檢查它是否是其中一個類的實例，如果不是，則在Trait范圍內查找匹配項。如果仍然找不到，則會發生編譯錯誤。此錯誤可以通過使用結構類型來解決，但由于推斷匿名類型可能會給程序員帶來意想不到的后果，因此它被設計為由程序員使用 `Structural` 顯式指定

## 類向上轉換

```python
i: Int
i as (Int or Str)
i as (1..10)
i as {I: Int | I >= 0}
```
<p align='center'>
    <a href='./15_quantified.md'>上一頁</a> | <a href='./17_type_casting.md'>下一頁</a>
</p>