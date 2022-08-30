# Special Type(Self, Super)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/special.md%26commit_hash%3Dae6d00168c17428bf967e44db3e6360e2471df8b)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/special.md&commit_hash=ae6d00168c17428bf967e44db3e6360e2471df8b)

`Self` represents itself types. It can be used simply as an alias, however note that its meaning changes in derived types (it refers to the derived own type).

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
