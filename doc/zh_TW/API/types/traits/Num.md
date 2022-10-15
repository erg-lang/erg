# Num

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Num.md%26commit_hash%3D14710744ed4c3aa29a43953366c67162bc157f7d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Num.md&commit_hash=14710744ed4c3aa29a43953366c67162bc157f7d)


`A<: B`表示類型A是類型B的子類型聲明。此外, 類型A此時稱為子類型, 類型B稱為廣義類型(超類型)。此外, 如果`A<: B`, 則類型為A的所有表達式都具有類型B的屬性。這稱為包含(subsumption)

Erg內置數字類型的類型關系如下:

- 布爾類型(Bool) <: 自然數類型(Nat) <: 整數類型(Int) <: 有理數類型(Ratio) <: 復合數類型(Complex)

指數文字是有理文字的另一種表示形式, 并且屬于同一類型

計算時，根據情況進行向上轉換(向下轉換)

> __注意__: 在當前的實現中，浮點類并不作為一個單獨的類存在，而是以與有理字面量相同的方式實現。將來, 這個浮點類將再次作為一個單獨的類實現, 以加快計算速度
> 此外，復雜對象目前是使用浮點對象實現的, 將來也會用有理字面量重寫

```python
>>> 1 + 1.0 # Nat(Int)+Ratio 向上轉換為 Ratio+Ratio
2.0 # Ratio
>>> 10.0 // 2 # Ratio//Nat(Int) 也向上轉換為 Ratio//Ratio. Ratio//Ratio 的結果是 Int
5 # Int(Nat)
>>> True == 1.0 # Bool==Ratio 向上轉換為 Ratio==Ratio
True
```

如果未指定類型, 則推斷它們以便它們向上轉換為相同類型
一般來說, 向下轉換是不安全的，轉換方法也很重要

以后不能重新定義類之間的轉換。只有在定義類時通過繼承指定超類時, 它才有資格進行強制轉換
此外，Trait不能被部分類型化, 除非它們在類定義時基本上“實現”。但是，這只能通過 [patch](../../../syntax/type/07_patch.md) 來完成

如果協變復合文字(例如數組文字)處于包含關系中, 則可以進行強制轉換
但是請注意，具有非退化的類型不能在 Erg 中強制轉換，即使它們處于包含關系中(有關詳細信息，請參閱 [degenerate](../../../syntax/type/advanced/variance.md))

## 定義

```python
Num R = Add(R) and Sub(R) and Mul(R) and Eq
Num = Num Self
```

## 父類(超類)

`Add`, `Sub`, `Mul` 和 `Eq`

## 方法

*`abs`
