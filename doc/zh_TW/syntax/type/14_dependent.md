# 依賴關係

依賴性可以說是 Erg 最大的特點。依賴關係是一種以值為參數的類型。通常的多相型只能將型作為自變量，但放鬆了其限制，就可以說是依存型。

依賴關係可以是（<gtr=“7”/>）。該類型不僅取決於內容的類型<gtr=“8”/>，而且取決於內容的數量<gtr=“9”/>。 <gtr=“10”/>包含類型為<gtr=“11”/>的對象。


```erg
a1 = [1, 2, 3]
assert a1 in [Nat; 3]
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4]
assert a1 + a2 in [Nat; 7]
```

如果在函數參數中傳遞的類型對象與返回類型相關聯，請使用。


```erg
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert narray(3) == [3, 3, 3]
```

定義依賴關係類型時，所有類型參數必須為常量。

依賴關係本身也存在於現有語言中，但 Erg 允許你定義依賴關係的過程方法。


```erg
x = 1
f x =
    print! f::x, module::x

# Phantom型は型引數と同じ値になるPhantomという屬性を持っている
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

可以通過應用方法來轉換可變依賴類型參數。轉換規範在中進行。


```erg
# `Id`は不変型なので遷移させることはできないことに注意
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(..., Impl := Phantom! State)
VM!().
    # 変わらない変數は`_`を渡せば省略可能, デフォルト引數にしておけば書く必要すらない
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!()
        self::set_phantom!("running")

# 型引數ごとに切り出すこともできる(定義されたモジュール內でのみ)
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

也可以通過合併或繼承現有類型來創建依賴類型。


```erg
MyArray(T, N) = Inherit [T; N]

# .arrayと連動してself: Self(T, N)の型が変わる
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```