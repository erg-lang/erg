# 量化依存型

Ergには量化型、依存型が存在します。すると当然、その二つを組み合わせた型を作ることができます。それが量化依存型です。

```erg
NonNullStr = |N: Nat| StrWithLen N | N != 0 # same as {S | N: Nat; S: StrWithLen N; N != 0}
NonEmptyArray = |N: Nat| [_; N | N > 0] # same as {A | N: Nat; A: Array(_, N); N > 0}
```

量化依存型の標準形は`T(A, ... | Pred)`です。`T`は型構築子、`A, B`は型引数、`Pred`は条件式です。

左辺値としての量化依存型は、元の型と同じモジュール内でのみメソッドを定義出来ます。

```erg
T A: Nat = Class ...
T(A).
    ...
T(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

右辺値としての量化依存型は、使用する型変数を型変数リスト(`||`)で宣言する必要がある。

```erg
a: |N: Nat| [T; N | N > 1]
```
