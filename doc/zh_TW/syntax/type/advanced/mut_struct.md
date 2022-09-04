# 可變結構類型

型是可以插入任意的<gtr=“5”/>型對象進行替換的箱型。


```erg
Particle! State: {"base", "excited"}! = Class(..., Impl := Phantom State)
Particle!.
    # このメソッドはStateを"base"から"excited"に遷移させる
    apply_electric_field!(ref! self("base" ~> "excited"), field: Vector) = ...
```

型雖然可以進行數據的替換，但不能改變其結構。如果用更接近現實的程序行為的說法，（堆上的）大小不能變更。這種類型稱為不變結構（可變）類型。

實際上，存在不變結構型無法表示的數據結構。例如，可變長度排列。型可以加入任意的<gtr=“8”/>對象，但不能替換為<gtr=“9”/>型對像等。

也就是說，長度不能改變。為了改變長度，必須改變型本身的結構。

實現那個的是可變結構（可變）型。


```erg
v = [Str; !0].new()
v.push! "Hello"
v: [Str; !1]
```

在可變結構型中，在可變化的類型自變量上添加。在上述情況下，可以將<gtr=“11”/>型改為<gtr=“12”/>型等。也就是說，可以改變長度。順便一提，型是型的糖衣句法。

可變結構型當然也可以用戶定義。但是，需要注意的是，與不變結構型在構成法方面有幾個不同。


```erg
Nil T = Class(Impl := Phantom T)
List T, !0 = Inherit Nil T
List T, N: Nat! = Class {head = T; rest = List(T, !N-1)}
List(T, !N).
    push! ref! self(N ~> N+1, ...), head: T =
        self.update! old -> Self.new {head; old}
```