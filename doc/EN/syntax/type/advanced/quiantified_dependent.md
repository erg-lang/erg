# Quantified Dependent Type

Erg has quantified and dependent types. Then naturally, it is possible to create a type that combines the two. That is the quantified dependent type.

```erg
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # same as {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # same as {A | N: Nat; A: Array(_, N); N > 0}
```

The standard form of a quantified dependent types are `K(A, ... | Pred)`. ``K`` is a type constructor, `A, B` are type arguments, and `Pred` is a conditional expression.

Quantified dependent types as a left-hand side value can only define methods in the same module as the original type.

```erg
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

Quantified dependent types as a right-hand side value require that the type variable to be used be declared in the type variable list (`||`).

```erg
# T is a concrete type
a: |N: Nat| [T; N | N > 1]
```
