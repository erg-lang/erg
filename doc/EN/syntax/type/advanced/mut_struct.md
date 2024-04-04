# Mutable Structure Type

The ``T!`` type is described as a box type that can be replaced by any ``T`` type object.

```python
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

```python
v = [Str; !0].new()
v.push! "Hello"
v: [Str; !1].
```

For mutable structure types, Mutable type arguments are marked with `!`. In the above case, the type `[Str; !0]` can be changed to `[Str; !1]` and so on. That is, the length can be changed.
Incidentally, the `[T; !N]` type is the sugar-coated syntax of the `ListWithLength!(T, !N)` type.

Mutable structure types can of course be user-defined. Note, however, that there are some differences from invariant structure types in terms of the construction method.

```python
Nil T = Class(Impl := Phantom T)
List T, !0 = Inherit Nil T
List T, N: Nat! = Class {head = T; rest = List(T, !N-1)}
List(T, !N).
    push! ref! self(N ~> N+1, ...), head: T =
        self.update! old -> Self.new {head; old}
```
