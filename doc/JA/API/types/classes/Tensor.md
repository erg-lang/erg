# Tensor Shape: [Nat; N]

  多次元配列を効率的に操作するためのクラス。多次元配列に対する積などの演算も定義する。
  Matrix, Vectorなどはこの型を継承している。

```erg
Tensor.arange(0..9) # Tensor [10]
```

* reshape(self, NewShape: [Nat; M]) -> Self NewShape

```erg
(1..9).into(Tensor).reshape [3, 3]
```

* identity i: Nat -> Self shape: [Nat; N]
* zeros(Shape: [Nat; N]) -> Self
* ones(Shape: [Nat; N]) -> Self

* diag

* linspace
* logspace
* geomspace
