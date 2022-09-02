# Mutable Structure Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/mut_struct.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/mut_struct.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

The ``T!`` type is described as a box type that can be replaced by any ``T`` type object.

```erg
Particle!State: {"base", "excited"}! = Class(... Impl := Phantom State)
Particle!
    # This method moves the State from "base" to "excited".
    apply_electric_field!(ref! self("base" ~> "excited"), field: Vector) = ...
```

The ``T!`` type can replace data, but it cannot change its structure.
More like the behavior of a real program, it cannot change its size (on the heap). Such a type is called an immutable structure (mutable) type.

In fact, there are data structures that cannot be represented by invariant structure types.
For example, a Mutable-length array. The `[T; N]!`type can contain objects of any `[T; N]`, but cannot be replaced by objects of type `[T; N+1]` and so on.

In other words, the length cannot be changed. To change the length, the structure of the type itself must be changed.

This is achieved by Mutable structure (mutable) types.

```erg
v = [Str; !0].new()
v.push! "Hello"
v: [Str; !1].
```

For mutable structure types, Mutable type arguments are marked with `!`. In the above case, the type `[Str; !0]` can be changed to `[Str; !1]` and so on. That is, the length can be changed.
Incidentally, the `[T; !N]` type is the sugar-coated syntax of the `ArrayWithLength!(T, !N)` type.

Mutable structure types can of course be user-defined. Note, however, that there are some differences from invariant structure types in terms of the construction method.

```erg
Nil T = Class(Impl := Phantom T)
List T, !0 = Inherit Nil T
List T, N: Nat! = Class {head = T; rest = List(T, !N-1)}
List(T, !N).
    push! ref! self(N ~> N+1, ...), head: T =
        self.update! old -> Self.new {head; old}
```
