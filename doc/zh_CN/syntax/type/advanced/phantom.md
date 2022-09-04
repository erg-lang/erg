# 幽灵类型（Phantom class）

幽灵型是只为给编译器注释而存在的标记trait。作为幽灵型的使用方法，来看清单的构成。


```erg
Nil = Class()
List T, 0 = Inherit Nil
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

这个代码是错误的。


```erg
3 | List T, 0 = Inherit Nil
                        ^^^
TypeConstructionError: since Nil does not have a parameter T, it is not possible to construct List(T, 0) with Nil
hint: use 'Phantom' trait to consume T
```

这个错误也就是当时不能进行<gtr=“7”/>的类型推论。在 Erg 中，类型自变量不能保持未使用状态。在这种情况下，无论什么都可以，所以必须在右边消耗型。如果大小为 0 的类型，例如长度为 0 的元组，则运行时没有开销，非常方便。


```erg
Nil T = Class((T; 0))
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

这个代码通过编译。但是有点棘手，意图很难理解，而且除了类型自变量是类型以外，不能使用。

这种时候正好是幽灵型。幽灵型是将大小为 0 的型一般化的型。


```erg
Nil T = Class(Impl := Phantom T)
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}

nil = Nil(Int).new()
assert nil.__size__ == 0
```

保留<gtr=“10”/>类型。但是实际上<gtr=“11”/>型的大小为 0，没有保存<gtr=“12”/>型的对象。

另外，除了类型以外还可以消耗任意类型自变量。在以下的例子中，<gtr=“16”/>保存了<gtr=“14”/>这一<gtr=“15”/>的子类型对象的类型自变量。这种情况下，<gtr=“17”/>也是不出现在对象实体中的哈利波特型变量。


```erg
VM! State: {"stopped", "running"}! = Class(..., Impl := Phantom! State)
VM!("stopped").
    start ref! self("stopped" ~> "running") =
        self.do_something!()
        self::set_phantom!("running")
```

通过<gtr=“19”/>方法或<gtr=“20”/>方法进行更新。这是<gtr=“21”/>（<gtr=“22”/>的可变版本）标准补丁提供的方法，其用法与可变类型的<gtr=“23”/>，<gtr=“24”/>相同。
