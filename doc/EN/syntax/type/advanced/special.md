# Special types (Self, Super)

`Self` represents its own type. You can just use it as an alias, but note that the meaning changes in derived types (refers to the own type).

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

`Super` represents the type of the base class. The method itself refers to the base class, but the instance uses its own type.

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

## special type variables

`Self` and `Super` can be used as type variables in structured types and traits. This refers to classes that are subtypes of that type. That is, `Self` in type `T` means `Self <: T`.

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