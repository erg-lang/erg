# Typeof, classof

是可以窥视 Erg 的类型推理系统的函数，其举动很复杂。


```erg
assert Typeof(1) == {I: Int | I == 1}
i: 1..3 or 5..10 = ...
assert Typeof(i) == {I: Int | (I >= 1 and I <= 3) or (I >= 5 and I <= 10)}

C = Class {i = Int}
I = C.new {i = 1}
assert Typeof(I) == {X: C | X == I}
J: C = ...
assert Typeof(J) == {i = Int}

assert {X: C | X == I} < C and C <= {i = Int}
```

函数返回的不是对象的类，而是结构类型。因此，对于<gtr=“6”/>类的实例<gtr=“7”/>，则为<gtr=“8”/>。关于值类，本来不存在对应的记录类型。为了解决这个问题，值类是具有<gtr=“9”/>属性的记录型。此外，不能访问该属性，也不能在用户定义类型中定义<gtr=“10”/>属性。


```erg
i: Int = ...
assert Typeof(i) == {__valueclass_tag__ = Phantom Int}
s: Str = ...
assert Typeof(s) == {__valueclass_tag__ = Phantom Str}
```

用输出的只是结构型。说明了结构型有属性型、筛子型和（真的）代数演算型。这些是独立的类型（存在推理的优先顺序），不发生推理的重解。属性型、代数运算型可能跨越多个类，而筛型是单一类的亚型。Erg 尽可能地将对象的类型作为筛子类型进行推论，当不能进行推论时，将筛子类型的基类扩大到结构化（后述）的类型。

## 结构化

所有类都可以转换为结构型。这被称为。可以通过函数获取类的结构化类型。如果用<gtr=“13”/>定义类（所有类都用这种形式定义），则<gtr=“14”/>。


```erg
C = Class {i = Int}
assert Structure(C) == {i = Int}
D = Inherit C
assert Structure(D) == {i = Int}
Nat = Class {I: Int | I >= 0}
assert Structure(Nat) == {I: Int | I >= 0}
Option T = Class (T or NoneType)
assert Structure(Option Int) == Or(Int, NoneType)
assert Structure(Option) # TypeError: only monomorphized types can be structurized
# 你实际上不能用 __valueclass_tag__ 定义一条记录，但在概念上
assert Structure(Int) == {__valueclass_tag__ = Phantom Int}
assert Structure(Str) == {__valueclass_tag__ = Phantom Str}
assert Structure((Nat, Nat)) == {__valueclass_tag__ = Phantom(Tuple(Nat, Nat))}
assert Structure(Nat -> Nat) == {__valueclass_tag__ = Phantom(Func(Nat, Nat))}
# 标记类也是带有 __valueclass_tag__ 的记录类型
M = Inherit Marker
assert Structure(M) == {__valueclass_tag__ = Phantom M}
D = Inherit(C and M)
assert Structure(D) == {i = Int; __valueclass_tag__ = Phantom M}
E = Inherit(Int and M)
assert Structure(E) == {__valueclass_tag__ = Phantom(And(Int, M))}
F = Inherit(E not M)
assert Structure(F) == {__valueclass_tag__ = Phantom Int}
```
