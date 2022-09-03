# 量化依赖性

Erg 有量化类型和依赖类型。这样自然就可以把这两个组合在一起做成模具了。那就是量化依赖型。


```erg
NonNullStr = |N: Nat| StrWithLen N | N != 0 # same as {S | N: Nat; S: StrWithLen N; N != 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # same as {A | N: Nat; A: Array(_, N); N > 0}
```

量化依赖性的标准形式是。其中<gtr=“5”/>是类型构建器，<gtr=“6”/>是类型参数，<gtr=“7”/>是条件表达式。

作为左边值的量化依赖类型只能在与原始类型相同的模块中定义方法。


```erg
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

量化依赖型作为右边值，必须在类型变量列表（）中声明要使用的类型变量。


```erg
# Tは具体的な型
a: |N: Nat| [T; N | N > 1]
```
