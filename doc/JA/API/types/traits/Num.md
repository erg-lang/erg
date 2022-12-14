# Num

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Num.md%26commit_hash%3D14710744ed4c3aa29a43953366c67162bc157f7d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Num.md&commit_hash=14710744ed4c3aa29a43953366c67162bc157f7d)

始めに用語の再確認をします。

`A <: B`は「型Aによる型Bの部分型宣言(subtype declaration)」を表します。またこの時の型Aを部分型(subtype)、型Bを汎化型(supertype)と言います。`A <: B`であるとき、型Aを持つ全ての式は型Bをの特性を持ちます。これは、形式的には`∀x: A (x: B)`を意味します(これはErgの有効な式ではありません)。

Erg組み込み数値型の型関係は以下のようになります。

- 真偽値型(Bool) <: 自然数型(Nat) <: 整数型(Int) <: 有理数型(Ratio) <: 複素数型(Complex)

また、指数リテラルは有理数リテラルの別表現であり、型としては同一になります。

計算する際には、適宜アップキャスト(ダウンキャスト)が行われます。

```python
>>> 1 + 1.0 # Nat(Int)+Ratioの型はRatio+Ratio型にアップキャストされる
2.0 # Ratio
>>> 10.0 // 2 # `Ratio型//Nat(Int)型`も同様に`Ratio型//Ratio型`にアップキャストされる。Ratio//Ratioの結果はInt
5 # Int(Nat)
>>> True == 1.0 # `Bool型==Ratio型`はRatio型==Ratio型`にアップキャストされる
True
```

> __Note__: 現在の実装では浮動小数クラスは独立したクラスとしては存在しておらず有理数リテラルと同じ実装になっています。将来的にはこの浮動小数クラスは高速計算用に独立したクラスとして再実装されます。
> また、複素数オブジェクトもまた現在浮動小数オブジェクトを使って実装されているため、同様に将来的には有理数リテラルによって書き直されます。

クラス同士のキャストは、後から定義しなおすことはできません。クラスを定義した際に継承でスーパークラスを指定した場合のみキャスト対象になります。
また、トレイトも基本的にクラス定義時に「実装」しなければトレイトの部分型付けすることができません。ただしこれは、[パッチ](../../../syntax/type/07_patch.md)を使うことで部分型付けとして見做すことができます。

共変な複合リテラルである配列リテラルなどは包摂関係にある場合にはキャスティングすることができます。
しかし、非変性を持つ型はergでは包摂関係にあってもキャストすることができないので注意が必要です(詳細は[変性](../../../syntax/type/advanced/variance.md)を参照してください)。

そして、ComplexはNumトレイトのサブクラスです。すなわち、Numは「数」全体を表すトレイトです。

## 定義

```python
Num R = Add(R) and Sub(R) and Mul(R) and Eq
Num = Num Self
```

## supers

`Add`, `Sub`, `Mul` and `Eq`

## メソッド

* `abs`
