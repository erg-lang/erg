# 操作员

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/operators.md%26commit_hash%3Df4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/operators.md&commit_hash=f4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)

## 中缀运算符

### `_+_`|R, O, A <: Add(R, O)|(x: A, y: R) -> O

执行加法

### `_-_`|R, O, S <: Sub(R, O)|(x: S, y: R) -> O

执行减法

### `*`|R, O, M <: Mul R, O|(x: M, y: R) -> O

执行乘法

### `/`|R, O, D <: Div(R, O)|(x: D, y: R) -> O

进行除法

## 中缀字母运算符

### `and`(x: Bool, y: Bool) -> Bool

执行 and 操作

### `or`(x: Bool, y: Bool) -> Bool

执行 and 操作

## 前缀运算符

### `+_`|T <: Num|(x: T) -> T

默认与 id 相同

### `-_`|T <: Num|(x: T) -> T.Neg

例如 Nat.`-`: Nat -> Neg 和返回值不同

### `!`|T <: Immut|(x: T) -> `T!`

从不可变对象创建可变对象
该运算符本身不是程序性的，可以在函数内部使用

### `..`|T <: Ord|(x: T) -> Range T

在 x 的末尾创建一个没有下限的 Range 对象
x..x 仅返回 x 作为迭代器

### `..<`|T <: Ord|(x: T) -> Range T

x..<x 产生一个空的 Range 对象，不产生任何迭代器

## 后缀运算符

解析中缀运算符失败时调用后缀运算符
也就是说，即使`x..`返回一个函数，`x..y`是`(..)(x, y)`而不是`(x..)(y)`

### |T <: Ord|(x: T)`..` -> Range T

创建一个从 x 开始没有上限的 Range 对象

### |T <: Ord|(x: T)`<..` -> Range T
