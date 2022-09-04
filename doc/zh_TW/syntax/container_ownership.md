# Subscript（下標訪問）

不同於常規方法。


```erg
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

請記住，不能在子例程的返回值中指定引用。在這裡，的類型顯然應該是<gtr=“6”/>（<gtr=“7”/>的類型取決於上下文）。因此，<gtr=“8”/>實際上是與<gtr=“9”/>相同的特殊語法的一部分。不像 Python，你不能過載。方法也不能再現<gtr=“10”/>行為。


```erg
C = Class {i = Int!}
C.get(ref self) =
    self::i # TypeError: `self::i` is `Int!` (require ownership) but `get` doesn't own `self`
C.steal(self) =
    self::i
# NG
C.new({i = 1}).steal().inc!() # OwnershipWarning: `C.new({i = 1}).steal()` is not owned by anyone
# hint: assign to a variable or use `uwn_do!`
# OK (assigning)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

此外，也可以剝奪所有權，但元素並不會因此而發生轉移。


```erg
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # OwnershipError: `a[0]` is moved to `i`
```