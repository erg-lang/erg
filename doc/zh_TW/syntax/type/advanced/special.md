# 特殊類型（Self，Super）

表示你的類型。你可以簡單地將其用作別名，但請注意，它在派生類型中的含義是不同的（指你自己的類型）。


```erg
@Inheritable
C = Class()
C.
    new_self() = Self.new()
    new_c() = C.new()
D = Inherit C

classof D.new_self() # D
classof D.new_c() # C
```

表示基類的類型。方法本身引用基類，而實例使用其類型。


```erg
@Inheritable
C = Class()

D = Inherit(C)
D.
    new_super() = Super.new()
    new_c() = C.new()

classof D.new_super() # D
classof D.new_c() # C
```

## 特殊類型變量

和<gtr=“7”/>可用作結構化任務中的類型變量。這是屬於該類型子類型的類。也就是說，在類型<gtr=“8”/>中，<gtr=“9”/>表示<gtr=“10”/>。


```erg
Add R = Trait {
    .AddO = Type
    .`_+_`: Self, R -> Self.AddO
}
ClosedAdd = Subsume Add(Self)

ClosedAddForInt = Patch(Int, Impl := ClosedAdd)
ClosedAddForInt.
    AddO = Int

assert 1 in Add(Int, Int)
assert 1 in ClosedAdd
assert Int < Add(Int, Int)
assert Int < ClosedAdd
```