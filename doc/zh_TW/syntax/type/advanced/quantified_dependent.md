# 量化依賴類型

Erg 有量化和依賴類型。 那么很自然地，就可以創建一個將兩者結合起來的類型。 那是量化的依賴類型。

```python
NonNullStr = |N: Nat| StrWithLen N | N ! = 0 # 同 {S | N: Nat; S: StrWithLen N; N ! = 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # 同 {A | N: Nat; A: Array(_, N); N > 0}
```

量化依賴類型的標準形式是“K(A, ... | Pred)”。 `K` 是類型構造函數，`A, B` 是類型參數，`Pred` 是條件表達式。

作為左值的量化依賴類型只能在與原始類型相同的模塊中定義方法。

```python
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

作為右值的量化依賴類型需要在類型變量列表 (`||`) 中聲明要使用的類型變量。

```python
# T 是具體類型
a: |N: Nat| [T; N | N > 1]
```
