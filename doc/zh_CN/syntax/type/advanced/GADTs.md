# Generalized Algebraic Data Types (GADTs)

Erg can create Generalized Algebraic Data Types (GADTs) by classifying Or types.

``` erg
Nil T = Class(Impl := Phantom T)
Cons T = Class {head = T; rest = List T}, Impl := Unpack
List T: Type = Class(Nil T or Cons T)
List.
    nil|T|() = Self(T).new Nil(T).new()
    cons head, rest | T = Self(T).new Cons(T).new(head, rest)
    head self = match self:
        {head; ...}: Cons_ -> head
        _: Nil -> panic "empty list"
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil.head() # RuntimeError: "empty list"
```

The reason we say `List.nil|T|() = ...` instead of `List(T).nil() = ...` is that we don't need to specify the type when using it.

``` erg
i = List.nil()
_: List Int = cons 1, i
```

The `List T` defined here is GADTs, but it's a naive implementation and doesn't show the true value of GADTs.
For example, the `.head` method above will throw a runtime error if the body is empty, but this check can be done at compile time.

``` erg
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

An example of GADTs that is often explained on the street is a list that can judge whether the contents are empty or not by type as above.
Erg can be further refined to define a list with length.

``` erg
List: (Type, Nat) -> Type
List T, 0 = Class(Impl := Phantom T)
List T, N = Class {head = T; rest = List(T, N-1)}, Impl := Unpack
List.
    nil|T|() = Self(T, 0).new Nil(T).new()
    cons head, rest | T, N = Self(T, N).new {head; rest}
List(_, N | N >= 1).
    head {head; ...} = head
List(_, N | N >= 2).
    pair {head = first; rest = {head = second; ...}} = [first, second]
{nil; cons; ...} = List

print! cons(1, cons(2, nil)).pair() # [1, 2]
print! cons(1, nil).pair() # TypeError
print! cons(1, nil).head() # 1
print! nil. head() # TypeError
```