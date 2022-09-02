# Generalized Algebraic Data Types (GADTs)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/GADTs.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/GADTs.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

Erg can create Generalized Algebraic Data Types (GADTs) by classifying Or (Union) types.

```erg
Nil T = Class(Impl := Phantom T)
Cons T = Class {head = T; rest = List T}, Impl := Unpack
List T: Type = Class(Nil T or Cons T)
List.
    nil|T|() = Self(T).new Nil(T).new()
    cons head, rest | T = Self(T).new Cons(T).new(head, rest)
    head self = match self:
        {head; ...} : Cons _ -> head
        _: Nil -> panic "empty list"
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil.head() # RuntimeError: "empty list"
```

not `List(T).nil() = ...`, but `List.nil|T|() = ...`. This is because the type specification is no longer needed when using it.

```erg
i = List.nil()
_: List Int = cons 1, i
```

The `List T` defined here is a GADTs, but it is a naive implementation and does not demonstrate the true value of GADTs.
For example, the `.head` method above will give a runtime error if the contents are empty, but this check can be done at compile time.

```erg
List: (Type, {"Empty", "Nonempty"}) -> Type
List T, "Empty" = Class(Impl := Phantom T)
List T, "Nonempty" = Class {head = T; rest = List(T, _)}, Impl := Unpack
List.
    nil|T|() = Self(T, "Empty").new Nil(T).new()
    cons head, rest | T = Self(T, "Nonempty").new {head; rest}
List(T, "Nonempty").
    head {head; ...} = head
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil().head() # TypeError
```

The GADTs example often described is a list whose contents can be judged as empty or not by type, as shown above.
Erg allows for further refinement, defining a list with length.

```erg
List: (Type, Nat) -> Type
List T, 0 = Class(Impl := Phantom T)
List T, N = Class {head = T; rest = List(T, N-1)}, Impl := Unpack
List.
    nil|T|() = Self(T, 0).new Nil(T).new()
    cons head, rest | T, N = Self(T, N).new {head; rest}
List(_, N | N >= 1).
    head {head; ...} = head
List(_, N | N >= 2).
    pair {head = first; rest = {head = second; ...}} = [first, second].
{nil; cons; ...} = List

print! cons(1, cons(2, nil)).pair() # [1, 2].
cons(1, nil).pair() # TypeError
cons(1, nil).head() # 1
print! nil.head() # TypeError
```
