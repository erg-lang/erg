# 结构

模块为它们提供代表代数结构和补丁的特征

* 成员

## 二进制运算

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

## 半群（一个二元运算的代数系统）

``` erg
SemiGroup Op: Kind 2 = Op(Self, Self)

IntIsSemiGroupAdd = Patch Int, Impl=SemiGroupAdd

Int <: SemiGroup Add
```

## 函子

``` erg
# * Identity law: x.map(id) == x
# * Composition law: x.map(f).map(g) == x.map(f.then g)
Functor = Trait {
    .map|T, U: Type| = (Self(T), T -> U) -> Self U
}
```

## 应用

``` erg
# * Identity law: x.app(X.pure(id)) == x
Applicative = Subsume Functor, Additional: {
    .pure|T: Type| = T -> Self T
    .app|T, U: Type| = (Self(T), Self(T -> U)) -> Self U
}
```

## 单子（交互式命令行工具以及面向对象的脚本技术）

``` erg
Monad = Subsume Applicative, Additional: {
    .bind|T, U: Type| = (Self(T), T -> Self U) -> Self U
}
```