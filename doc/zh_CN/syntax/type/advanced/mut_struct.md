# 可变结构类型

型是可以插入任意的<gtr=“5”/>型对象进行替换的箱型。


```erg
Particle! State: {"base", "excited"}! = Class(..., Impl := Phantom State)
Particle!.
    # このメソッドはStateを"base"から"excited"に遷移させる
    apply_electric_field!(ref! self("base" ~> "excited"), field: Vector) = ...
```

型虽然可以进行数据的替换，但不能改变其结构。如果用更接近现实的程序行为的说法，（堆上的）大小不能变更。这种类型称为不变结构（可变）类型。

实际上，存在不变结构型无法表示的数据结构。例如，可变长度排列。型可以加入任意的<gtr=“8”/>对象，但不能替换为<gtr=“9”/>型对象等。

也就是说，长度不能改变。为了改变长度，必须改变型本身的结构。

实现那个的是可变结构（可变）型。


```erg
v = [Str; !0].new()
v.push! "Hello"
v: [Str; !1]
```

在可变结构型中，在可变化的类型自变量上添加。在上述情况下，可以将<gtr=“11”/>型改为<gtr=“12”/>型等。也就是说，可以改变长度。顺便一提，型是型的糖衣句法。

可变结构型当然也可以用户定义。但是，需要注意的是，与不变结构型在构成法方面有几个不同。


```erg
Nil T = Class(Impl := Phantom T)
List T, !0 = Inherit Nil T
List T, N: Nat! = Class {head = T; rest = List(T, !N-1)}
List(T, !N).
    push! ref! self(N ~> N+1, ...), head: T =
        self.update! old -> Self.new {head; old}
```
