# Num

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Num.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Num.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

始めに用語の再確認をします。

`A <: B`は型Aは型Bの部分型付け(subtyping)であることを表します。またこの時の型Aを部分型(subtype)、型Bを汎化型(supertype)と言います。更に、`A <: B`ならば型Aを持つ全ての式は型Bをの特性を持ちます。これを包摂(subsumption)と言います。

では既存の数値型の関係性について説明していきます。

- 自然数リテラル(Nat Literal) <: 整数リテラル(Int Literal) <: 有理数リテラル(Ratio Literal(指数リテラル(Exponential Literal))) <: 浮動小数オブジェクト(Float Object)

- 有理数リテラル <: 複素数オブジェクト(Complement Object)

- 真偽値オブジェクト(Boolean Object) <: 整数リテラル

このようにリテラルは浮動小数オブジェクトを汎化型として持ち、有理数リテラルの汎化型として複素数オブジェクトを持ちます。また、整数リテラルは真偽値オブジェクトを部分型として持ちます。
これにより数値計算する際には型が指定されていなければ、それぞれの型が部分型であればアップキャスト(ダウンキャスト)されます。
指数リテラルは有理数リテラルの別表現であり、型としては同一になります。

> __Note__: 現在の実装では浮動小数クラスは独立したクラスとしては存在しておらず有理数リテラルと同じ実装になっています。将来的にはこの浮動小数クラスは高速計算用に独立したクラスとして再実装されます。
> また、複素数オブジェクトもまた現在浮動小数オブジェクトを使って実装されているため、同様に将来的には有理数リテラルによって書き直されます。

```python
>>> 1 + 1.0 # Nat(Int)+Ratioの型はRatio+Ratio型にアップキャストされる
2.0 # Float
>>> 10.0 // 2 # `Ratio型//Nat(Int)型`も同様に`Ratio型//Ratio型`にアップキャストされ、その後Ratio型がInt型にダウンキャストされる
5 # Int(Nat)
>>> True == 1.0 # `Bool型==Ratio型`はRatio型==Ratio型`にアップキャストされる
True
```

これは厳密な型推定により引き起こされます。
型は同じ型同士でしか計算をすることはできません。
そのため、型を指定していなければ、同じ型同士になるようにアップキャストされるように型が推論されます。
一般的に、ダウンキャストは安全ではなく、変換方法も自明でありません。今回のように`//`によって少数部分が切り捨てられることが変換として自明な場合はダウンキャストすることができます。

クラス同士のキャストは、後から定義仕直すことはできません。クラスを定義した際に継承でスーパークラスを指定した場合のみキャスト対象になります。
また、トレイトも基本的にクラス定義時に「実装」しなければトレイトの部分型付けすることができません。ただしこれは、[パッチ](../../../syntax/type/07_patch.md)を使うことで部分型付けとして見做すことができます。

共変な複合リテラルである配列リテラルなどは包摂関係にある場合にはキャスティングすることができます。
しかし、非変性を持つ型はergでは包摂関係にあってもキャストすることができないので注意が必要です(詳細は[変性](../../../syntax/type/advanced/variance.md)を参照してください)。

## 定義

```python
Num R = Add(R) and Sub(R) and Mul(R) and Eq
Num = Num Self
```

## supers

`Add`, `Sub`, `Mul` and `Eq`

## メソッド

* `abs`
