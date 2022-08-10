# Float size

実数(小数を含む数)を表す型です。IEEE 754に準拠した浮動小数点数を表し、他の言語では一般的にfloatに相当する型です。
Float sizeのsizeは、8(1byte)~128(16byte)となります。単にFloatとした場合`Float 64`を表します。
Ergでの0.1は実はFloat型ではなく、Ratio型に属します。Float型のリテラルは存在せず、`(Ratioオブジェクト)f64`(e.g. (1/2)f64, 15f64)で生成します。f64は実数の1に対応します。

## supers

Complex and Ord

## methods

* sgn(self) -> {-1, 0, 1}
  符号を返す。

* truncate(self) -> Int
  自身に最も近い整数を返す。

* separate(self) -> [Str]
* separate(self, dight: Nat) -> [Str]
  dight桁ごとに区切る。引数なしだと3。
