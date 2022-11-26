# 演算子

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/operators.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/operators.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

## 中置演算子

### `_+_`|R, O, A <: Add(R, O)|(x: A, y: R) -> O

加算を実行する。

### `_-_`|R, O, S <: Sub(R, O)|(x: S, y: R) -> O

減算を実行する。

### `*`|R, O, M <: Mul R, O|(x: M, y: R) -> O

乗算を実行する。

### `/`|R, O, D <: Div(R, O)|(x: D, y: R) -> O

除算を実行する。

## 中置アルファベット演算子

### `and`(x: Bool, y: Bool) -> Bool

and演算を実行する。

### `or`(x: Bool, y: Bool) -> Bool

and演算を実行する。

## 前置演算子

### `+_`|T <: Num|(x: T) -> T

デフォルトではidと同じ。

### `-_`|T <: Num|(x: T) -> T.Neg

例えば、Nat.`-`: Nat -> Negとなり、戻り値が違う。

### `!`|T <: Immut|(x: T) -> `T!`

不変オブジェクトから可変オブジェクトを生成する。
この演算子自体はProceduralではなく、関数内でも使える。

### `..`|T <: Ord|(x: T) -> Range T

x終わりで下界のないRangeオブジェクトを生成する。
x..xはイテレータとしてxのみ返す。

### `..<`|T <: Ord|(x: T) -> Range T

x..<xは空Rangeオブジェクトになり、イテレータとして何も生成しない。

## 後置演算子

後置演算子は構文解析上中置演算子の解析が失敗した際に呼び出される。
すなわち、`x..`が関数を返したとしても`x.. y`は`(..)(x, y)`であり`(x..)(y)`とは解釈されない。

### |T <: Ord|(x: T)`..` -> Range T

x始まりで上界のないRangeオブジェクトを生成する。

### |T <: Ord|(x: T)`<..` -> Range T
