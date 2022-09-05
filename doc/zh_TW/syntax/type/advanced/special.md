# 特殊類型(Self、Super)

`Self` 代表它自己的類型。 您可以將其用作別名，但請注意派生類型的含義會發生變化(指的是自己的類型)。

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

`Super` 表示基類的類型。方法本身引用基類，但實例使用自己的類型。

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

## 特殊類型變量

`Self` 和 `Super` 可以用作結構化類型和特征中的類型變量。 這指的是作為該類型子類型的類。 也就是說，`T` 類型中的`Self` 表示`Self <: T`。

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