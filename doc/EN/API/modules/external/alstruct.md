# alstruct

Modules that provide traits representing algebraic structures and patches for them.

* members

## BinOp

``` erg
BinOp Op: Kind 2 = Subsume Op(Self, Self.ReturnTypeOf Op), Additional: {
    .ReturnTypeof = TraitType -> Type
}

Nat <: BinOp Add
assert Nat. ReturnTypeof(Add) == Nat
assert Nat. ReturnTypeof(Sub) == Int
assert Nat. ReturnTypeof(Mul) == Nat
assert Nat.ReturnTypeof(Div) == Positive Ratio
```

## SemiGroup

``` erg
SemiGroup Op: Kind 2 = Op(Self, Self)

IntIsSemiGroupAdd = Patch Int, Impl=SemiGroupAdd

Int <: SemiGroup Add
```

## Functors

``` erg
# * Identity law: x.map(id) == x
# * Composition law: x.map(f).map(g) == x.map(f.then g)
Functor = Trait {
    .map|T, U: Type| = (Self(T), T -> U) -> Self U
}
```

## Applicative

``` erg
# * Identity law: x.app(X.pure(id)) == x
Applicative = Subsume Functor, Additional: {
    .pure|T: Type| = T -> Self T
    .app|T, U: Type| = (Self(T), Self(T -> U)) -> Self U
}
```

## Monad

``` erg
Monad = Subsume Applicative, Additional: {
    .bind|T, U: Type| = (Self(T), T -> Self U) -> Self U
}
```