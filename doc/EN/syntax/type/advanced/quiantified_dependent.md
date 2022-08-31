# Quantified Dependent Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/quiantified_dependent.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/quiantified_dependent.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

Erg has quantified and dependent types. Then naturally, it is possible to create a type that combines the two. That is the quantified dependent type.

```erg
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # same as {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # same as {A | N: Nat; A: Array(_, N); N > 0}
```

The standard form of quantified dependent types are `K(A, ... | Pred)`. ``K`` is a type constructor, `A, B` are type arguments, and `Pred` is a conditional expression.

Quantified dependent types as left-hand side values can only define methods in the same module as the original type.

```erg
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

Quantified dependent types as right-hand side values require that the type variable to be used be declared in the type variable list (`||`).

```erg
# T is a concrete type
a: |N: Nat| [T; N | N > 1]
```
