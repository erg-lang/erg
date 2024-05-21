# 操作員

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/operators.md%26commit_hash%3Df4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/operators.md&commit_hash=f4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)

## 中綴運算符

### `_+_`|R, O, A <: Add(R, O)|(x: A, y: R) -> O

執行加法

### `_-_`|R, O, S <: Sub(R, O)|(x: S, y: R) -> O

執行減法

### `*`|R, O, M <: Mul R, O|(x: M, y: R) -> O

執行乘法

### `/`|R, O, D <: Div(R, O)|(x: D, y: R) -> O

進行除法

## 中綴字母運算符

### `and`(x: Bool, y: Bool) -> Bool

執行 and 操作

### `or`(x: Bool, y: Bool) -> Bool

執行 and 操作

## 前綴運算符

### `+_`|T <: Num|(x: T) -> T

默認與 id 相同

### `-_`|T <: Num|(x: T) -> T.Neg

例如 Nat.`-`: Nat -> Neg 和返回值不同

### `!`|T <: Immut|(x: T) -> `T!`

從不可變對象創建可變對象
該運算符本身不是程序性的，可以在函數內部使用

### `..`|T <: Ord|(x: T) -> Range T

在 x 的末尾創建一個沒有下限的 Range 對象
x..x 僅返回 x 作為迭代器

### `..<`|T <: Ord|(x: T) -> Range T

x..<x 產生一個空的 Range 對象，不產生任何迭代器

## 后綴運算符

解析中綴運算符失敗時調用后綴運算符
也就是說，即使`x..`返回一個函數，`x..y`是`(..)(x, y)`而不是`(x..)(y)`

### |T <: Ord|(x: T)`..` -> Range T

創建一個從 x 開始沒有上限的 Range 對象

### |T <: Ord|(x: T)`<..` -> Range T
