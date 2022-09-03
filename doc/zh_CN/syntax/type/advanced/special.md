# 特殊类型（Self，Super）

表示你的类型。你可以简单地将其用作别名，但请注意，它在派生类型中的含义是不同的（指你自己的类型）。


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

表示基类的类型。方法本身引用基类，而实例使用其类型。


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

## 特殊类型变量

和<gtr=“7”/>可用作结构化任务中的类型变量。这是属于该类型子类型的类。也就是说，在类型<gtr=“8”/>中，<gtr=“9”/>表示<gtr=“10”/>。


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
