# Quantified Dependent Type

Erg has quantified and dependent types. Then naturally, it is possible to create a type that combines the two. That is the quantified dependent type.

```python
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # same as {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # same as {A | N: Nat; A: Array(_, N); N > 0}
```

The standard form of quantified dependent types are `K(A, ... | Pred)`. ``K`` is a type constructor, `A, B` are type arguments, and `Pred` is a conditional expression.

Quantified dependent types as left-hand side values can only define methods in the same module as the original type.

```python
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

Quantified dependent types as right-hand side values require that the type variable to be used be declared in the type variable list (`||`).

```python
# T is a concrete type
a: |N: Nat| [T; N | N > 1]
```
