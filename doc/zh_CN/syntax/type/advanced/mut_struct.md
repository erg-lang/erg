# 可变结构类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/mut_struct.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/mut_struct.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

`T!` 类型被描述为可以被任何 `T` 类型对象替换的盒子类型。

```python
Particle!State: {"base", "excited"}! = Class(... Impl := Phantom State)
Particle!
    # 此方法将状态从"base"移动到"excited"
    apply_electric_field!(ref! self("base" ~> "excited"), field: Vector) = ...
```

`T!` 类型可以替换数据，但不能改变其结构。
更像是一个真实程序的行为，它不能改变它的大小(在堆上)。 这样的类型称为不可变结构(mutable)类型。

事实上，有些数据结构不能用不变的结构类型来表示。
例如，可变长度数组。 `[T; N]!`类型可以包含任何`[T; N]`，但不能被`[T; N+1]` 等等。

换句话说，长度不能改变。 要改变长度，必须改变类型本身的结构。

这是通过可变结构(可变)类型实现的。

```python
v = [Str; !0].new()
v.push! "Hello"
v: [Str; !1].
```

对于可变结构类型，可变类型参数用 `!` 标记。 在上述情况下，类型 `[Str; !0]` 可以更改为 `[Str; !1]` 等等。 即，可以改变长度。
顺便说一句，`[T; !N]` 类型是 `ArrayWithLength!(T, !N)` 类型的糖衣语法。

可变结构类型当然可以是用户定义的。 但是请注意，在构造方法方面与不变结构类型存在一些差异。

```python
Nil T = Class(Impl := Phantom T)
List T, !0 = Inherit Nil T
List T, N: Nat! = Class {head = T; rest = List(T, !N-1)}
List(T, !N).
    push! ref! self(N ~> N+1, ...), head: T =
        self.update! old -> Self.new {head; old}
```
