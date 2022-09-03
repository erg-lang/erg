# Subscript（下标访问）

不同于常规方法。


```erg
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

请记住，不能在子例程的返回值中指定引用。在这里，的类型显然应该是<gtr=“6”/>（<gtr=“7”/>的类型取决于上下文）。因此，<gtr=“8”/>实际上是与<gtr=“9”/>相同的特殊语法的一部分。不像 Python，你不能过载。方法也不能再现<gtr=“10”/>行为。


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

此外，也可以剥夺所有权，但元素并不会因此而发生转移。


```erg
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # OwnershipError: `a[0]` is moved to `i`
```
