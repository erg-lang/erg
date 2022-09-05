# 特殊类型(Self、Super)

`Self` 代表它自己的类型。 您可以将其用作别名，但请注意派生类型的含义会发生变化(指的是自己的类型)。

```python
@Inheritable
C = Class()
C.
    new_self() = Self. new()
    new_c() = C.new()
D = Inherit C

classof D. new_self() # D
classof D. new_c() # C
```

`Super` 表示基类的类型。方法本身引用基类，但实例使用自己的类型。

```python
@Inheritable
C = Class()

D = Inherit(C)
D.
    new_super() = Super.new()
    new_c() = C.new()

classof D. new_super() # D
classof D. new_c() # C
```

## 特殊类型变量

`Self` 和 `Super` 可以用作结构化类型和特征中的类型变量。 这指的是作为该类型子类型的类。 也就是说，`T` 类型中的`Self` 表示`Self <: T`。

```python
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