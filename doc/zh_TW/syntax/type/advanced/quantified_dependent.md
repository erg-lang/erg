# 量化依賴性

Erg 有量化類型和依賴類型。這樣自然就可以把這兩個組合在一起做成模具了。那就是量化依賴型。


```erg
NonNullStr = |N: Nat| StrWithLen N | N != 0 # same as {S | N: Nat; S: StrWithLen N; N != 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # same as {A | N: Nat; A: Array(_, N); N > 0}
```

量化依賴性的標準形式是。其中<gtr=“5”/>是類型構建器，<gtr=“6”/>是類型參數，<gtr=“7”/>是條件表達式。

作為左邊值的量化依賴類型只能在與原始類型相同的模塊中定義方法。


```erg
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

量化依賴型作為右邊值，必須在類型變量列表（）中聲明要使用的類型變量。


```erg
# Tは具體的な型
a: |N: Nat| [T; N | N > 1]
```