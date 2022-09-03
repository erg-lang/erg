# 依赖关系

依赖性可以说是 Erg 最大的特点。依赖关系是一种以值为参数的类型。通常的多相型只能将型作为自变量，但放松了其限制，就可以说是依存型。

依赖关系可以是（<gtr=“7”/>）。该类型不仅取决于内容的类型<gtr=“8”/>，而且取决于内容的数量<gtr=“9”/>。<gtr=“10”/>包含类型为<gtr=“11”/>的对象。


```erg
a1 = [1, 2, 3]
assert a1 in [Nat; 3]
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4]
assert a1 + a2 in [Nat; 7]
```

如果在函数参数中传递的类型对象与返回类型相关联，请使用。


```erg
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert narray(3) == [3, 3, 3]
```

定义依赖关系类型时，所有类型参数必须为常量。

依赖关系本身也存在于现有语言中，但 Erg 允许你定义依赖关系的过程方法。


```erg
x = 1
f x =
    print! f::x, module::x

# Phantom型は型引数と同じ値になるPhantomという属性を持っている
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

可以通过应用方法来转换可变依赖类型参数。转换规范在中进行。


```erg
# `Id`は不変型なので遷移させることはできないことに注意
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(..., Impl := Phantom! State)
VM!().
    # 変わらない変数は`_`を渡せば省略可能, デフォルト引数にしておけば書く必要すらない
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!()
        self::set_phantom!("running")

# 型引数ごとに切り出すこともできる(定義されたモジュール内でのみ)
VM!.new() = VM!(!"stopped", 1).new()
VM!("running" ~> "running").stop! ref! self =
    self.close_something!()
    self::set_phantom!("stopped")

vm = VM!.new()
vm.start!()
vm.stop!()
vm.stop!() # TypeError: VM!(!"stopped", 1) doesn't have .stop!()
# hint: VM!(!"running", 1) has .stop!()
```

也可以通过合并或继承现有类型来创建依赖类型。


```erg
MyArray(T, N) = Inherit [T; N]

# .arrayと連動してself: Self(T, N)の型が変わる
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```
