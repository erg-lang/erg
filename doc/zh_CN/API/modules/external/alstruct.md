# alstruct

提供表示代数结构的托盘和相应补丁的模块。

* member

## binop

``` erg
Kind 2 = Subsume Op(Self, Self. returntypeof Op)， Additional: {
.ReturnTypeof = TraitType -> Type
}

Nat <: BinOp Add
assert Nat. returntypeof (Add) == Nat
assert Nat.ReturnTypeof(Sub) == Int
assert Nat. returntypeof (Mul) == Nat
assert Nat.ReturnTypeof(Div) == Positive Ratio
```

## semigroup

``` erg
SemiGroup Op: Kind 2 = Op(Self, Self)

IntIsSemiGroupAdd = Patch Int, Impl=SemiGroup Add

Int <: SemiGroup Add
```

## functor

``` erg
## * Identity law: x.map(id) == x
## * Composition law: x.map(f).map(g) == x.map(f.then g)
Functor = Trait {
. map | t, u: type | = (self (t), t - > u) - > self u
}
```

## applicative

``` erg
## * Identity law: x.app(x. pure(id)) == x
Applicative = Subsume Functor, Additional: {
t: . pure | type | = t - > self t
. app | t, u: type | = (self (t), self (t - > u)) - > self u
}
```

## monad

``` erg
Monad = Subsume Applicative, Additional: {
. bind | t, u: type | = (self (t), t - > self u) - > self u
}
```