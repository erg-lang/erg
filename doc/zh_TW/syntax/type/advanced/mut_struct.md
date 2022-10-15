# 可變結構類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/mut_struct.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/mut_struct.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

`T!` 類型被描述為可以被任何 `T` 類型對象替換的盒子類型

```python
Particle!State: {"base", "excited"}! = Class(... Impl := Phantom State)
Particle!
    # 此方法將狀態從"base"移動到"excited"
    apply_electric_field!(ref! self("base" ~> "excited"), field: Vector) = ...
```

`T!` 類型可以替換數據，但不能改變其結構
更像是一個真實程序的行為，它不能改變它的大小(在堆上)。這樣的類型稱為不可變結構(mutable)類型

事實上，有些數據結構不能用不變的結構類型來表示
例如，可變長度數組。`[T; N]!`類型可以包含任何`[T; N]`，但不能被`[T; N+1]` 等等

換句話說，長度不能改變。要改變長度，必須改變類型本身的結構

這是通過可變結構(可變)類型實現的

```python
v = [Str; !0].new()
v.push! "Hello"
v: [Str; !1].
```

對于可變結構類型，可變類型參數用 `!` 標記。在上述情況下，類型 `[Str; !0]` 可以更改為 `[Str; !1]` 等等。即，可以改變長度
順便說一句，`[T; !N]` 類型是 `ArrayWithLength!(T, !N)` 類型的糖衣語法

可變結構類型當然可以是用戶定義的。但是請注意，在構造方法方面與不變結構類型存在一些差異

```python
Nil T = Class(Impl := Phantom T)
List T, !0 = Inherit Nil T
List T, N: Nat! = Class {head = T; rest = List(T, !N-1)}
List(T, !N).
    push! ref! self(N ~> N+1, ...), head: T =
        self.update! old -> Self.new {head; old}
```
