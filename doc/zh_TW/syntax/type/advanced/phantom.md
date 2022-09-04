# 幽靈類型（Phantom class）

幽靈型是只為給編譯器註釋而存在的標記trait。作為幽靈型的使用方法，來看清單的構成。


```erg
Nil = Class()
List T, 0 = Inherit Nil
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

這個代碼是錯誤的。


```erg
3 | List T, 0 = Inherit Nil
                        ^^^
TypeConstructionError: since Nil does not have a parameter T, it is not possible to construct List(T, 0) with Nil
hint: use 'Phantom' trait to consume T
```

這個錯誤也就是當時不能進行<gtr=“7”/>的類型推論。在 Erg 中，類型自變量不能保持未使用狀態。在這種情況下，無論什麼都可以，所以必須在右邊消耗型。如果大小為 0 的類型，例如長度為 0 的元組，則運行時沒有開銷，非常方便。


```erg
Nil T = Class((T; 0))
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

這個代碼通過編譯。但是有點棘手，意圖很難理解，而且除了類型自變量是類型以外，不能使用。

這種時候正好是幽靈型。幽靈型是將大小為 0 的型一般化的型。


```erg
Nil T = Class(Impl := Phantom T)
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}

nil = Nil(Int).new()
assert nil.__size__ == 0
```

保留<gtr=“10”/>類型。但是實際上<gtr=“11”/>型的大小為 0，沒有保存<gtr=“12”/>型的對象。

另外，除了類型以外還可以消耗任意類型自變量。在以下的例子中，<gtr=“16”/>保存了<gtr=“14”/>這一<gtr=“15”/>的子類型對象的類型自變量。這種情況下，<gtr=“17”/>也是不出現在對象實體中的哈利波特型變量。


```erg
VM! State: {"stopped", "running"}! = Class(..., Impl := Phantom! State)
VM!("stopped").
    start ref! self("stopped" ~> "running") =
        self.do_something!()
        self::set_phantom!("running")
```

通過<gtr=“19”/>方法或<gtr=“20”/>方法進行更新。這是<gtr=“21”/>（<gtr=“22”/>的可變版本）標準補丁提供的方法，其用法與可變類型的<gtr=“23”/>，<gtr=“24”/>相同。