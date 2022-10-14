# Float size

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Float.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Float.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

表示實數(包含小數的數)的類型。符合IEEE 754的浮點數，在其他語言中一般是float的類型
Float的大小為8(1byte)~128(16byte)。如果只是Float，則表示`Float64`
Erg 中的 0.1 實際上屬于 Ratio 類型，而不是 Float 類型。沒有浮點類型字面量，它是由 `(Ratio object)f64` 生成的(例如 (1/2)f64, 15f64)。 f64 對應實數 1

## 父類

Complex 和 Ord

## 方法

* sgn(self) -> {-1, 0, 1}
  返回標志

* truncate(self) -> Int
  返回最接近自身的整數

* separate(self) -> [Str]
* separate(self, dight: Nat) -> [Str]
  按digit位數劃分。沒有自變量
