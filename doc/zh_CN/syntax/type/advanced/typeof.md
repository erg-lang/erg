# Typeof, classof

`Typeof` 是一个可以窥探 Erg 类型推断系统的函数，它的行为很复杂

```python
assert Typeof(1) == {I: Int | I == 1}
i: 1..3 or 5..10 = ...
assert Typeof(i) == {I: Int | (I >= 1 and I <= 3) or (I >= 5 and I <= 10)}

C = Class {i = Int}
I = C. new {i = 1}
assert Typeof(I) == {X: C | X == I}
J: C = ...
assert Typeof(J) == {i = Int}

assert {X: C | X == I} < C and C <= {i = Int}
```

`Typeof` 函数返回派生类型，而不是对象的类。
因此，例如 `C = Class T` 类的`I: C`，`Typeof(I) == T`。
值类没有对应的记录类型。 为了解决这个问题，值类应该是具有 `__valueclass_tag__` 属性的记录类型。
请注意，您不能访问此属性，也不能在用户定义的类型上定义 `__valueclass_tag__` 属性。

```python
i: Int = ...
assert Typeof(i) == {__valueclass_tag__ = Phantom Int}
s: Str = ...
assert Typeof(s) == {__valueclass_tag__ = Phantom Str}
```

`Typeof` 仅输出结构化类型。 我解释说结构化类型包括属性类型、筛类型和（真正的）代数类型。
这些是独立的类型（存在推理优先级），不会发生推理冲突。
属性类型和代数类型可以跨越多个类，而筛类型是单个类的子类型。
Erg 尽可能将对象类型推断为筛类型，如果不可能，则将筛基类扩展为结构化类型（见下文）。

## 结构化的

所有类都可以转换为派生类型。 这称为 __结构化__。 类的结构化类型可以通过 `Structure` 函数获得。
如果一个类是用`C = Class T`定义的（所有类都以这种形式定义），那么`Structure(C) == T`。

```python
C = Class {i = Int}
assert Structure(C) == {i = Int}
D = Inherit C
assert Structure(D) == {i = Int}
Nat = Class {I: Int | I >= 0}
assert Structure(Nat) == {I: Int | I >= 0}
Option T = Class (T or NoneType)
assert Structure(Option Int) == Or(Int, NoneType)
assert Structure(Option) # 类型错误：只能构造单态类型
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