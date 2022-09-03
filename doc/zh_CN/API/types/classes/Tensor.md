# Tensor Shape: [Nat; N]

  用于有效操作多维数组的类。 它还定义了诸如多维数组上的乘法之类的操作
  Matrix、Vector 等都继承自该类型

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
