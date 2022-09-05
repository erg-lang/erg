# Float size

表示实数(包含小数的数)的类型。符合IEEE 754的浮点数，在其他语言中一般是float的类型。
Float的大小为8(1byte)~128(16byte)。如果只是Float，则表示`Float64`。
Erg 中的 0.1 实际上属于 Ratio 类型，而不是 Float 类型。没有浮点类型字面量，它是由 `(Ratio object)f64` 生成的(例如 (1/2)f64, 15f64)。 f64 对应实数 1

## 父类

Complex 和 Ord

## 方法

* sgn(self) -> {-1, 0, 1}
  返回标志

* truncate(self) -> Int
  返回最接近自身的整数

* separate(self) -> [Str]
* separate(self, dight: Nat) -> [Str]
  按digit位数划分。没有自变量
