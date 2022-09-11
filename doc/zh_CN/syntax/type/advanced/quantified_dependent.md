# 量化依赖类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/quantified_dependent.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/quantified_dependent.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

Erg 有量化和依赖类型。 那么很自然地，就可以创建一个将两者结合起来的类型。 那是量化的依赖类型。

```python
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # 同 {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # 同 {A | N: Nat; A: Array(_, N); N > 0}
```

量化依赖类型的标准形式是"K(A, ... | Pred)"。 `K` 是类型构造函数，`A, B` 是类型参数，`Pred` 是条件表达式。

作为左值的量化依赖类型只能在与原始类型相同的模块中定义方法。

```python
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

作为右值的量化依赖类型需要在类型变量列表 (`||`) 中声明要使用的类型变量。

```python
# T 是具体类型
a: |N: Nat| [T; N | N > 1]
```
