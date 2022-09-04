# 量化依赖类型

Erg 有量化和依赖类型。 那么很自然地，就可以创建一个将两者结合起来的类型。 那是量化的依赖类型。

```python
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # 同 {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # 同 {A | N: Nat; A: Array(_, N); N > 0}
```

量化依赖类型的标准形式是“K(A, ... | Pred)”。 `K` 是类型构造函数，`A, B` 是类型参数，`Pred` 是条件表达式。

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
