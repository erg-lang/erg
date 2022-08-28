# Special Type(Self, Super)

`Self` represents itself types.
It can be used simply as an alias, however note that its meaning changes in derived types (it refers to the derived own type).

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

`Super` represents types of base classes. The method itself refers to the base class, however the instance uses its own type.

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

## Special Type Variable

`Self` and `Super` can be used as type variables in structural types and traits. It refers to a class, which is a subtype of that type. That is, `Self` in type `T` means `Self <: T`.

```erg
Add R = Trait {
    .AddO = Type
    .`_+_`: Self, R -> Self.AddO
}
ClosedAdd = Subsume Add(Self)

ClosedAddForInt = Patch(Int, Impl: ClosedAdd)
ClosedAddForInt.
    AddO = Int

assert 1 in Add(Int, Int)
assert 1 in ClosedAdd
assert Int < Add(Int, Int)
assert Int < ClosedAdd
```
