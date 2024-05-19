# 量化依賴類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/quantified_dependent.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/quantified_dependent.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

Erg 有量化和依賴類型。那么很自然地，就可以創建一個將兩者結合起來的類型。那是量化的依賴類型

```python
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # 同 {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyList = |N: Nat| [_; N | N > 0] # 同 {A | N: Nat; A: List(_, N); N > 0}
```

量化依賴類型的標準形式是"K(A, ... | Pred)"。`K` 是類型構造函數，`A, B` 是類型參數，`Pred` 是條件表達式

作為左值的量化依賴類型只能在與原始類型相同的模塊中定義方法

```python
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

作為右值的量化依賴類型需要在類型變量列表 (`||`) 中聲明要使用的類型變量

```python
# T 是具體類型
a: |N: Nat| [T; N | N > 1]
```
