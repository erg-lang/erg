# Tensor Shape: [Nat; N]

  用于有效操作多維數組的類。 它還定義了諸如多維數組上的乘法之類的操作
  Matrix、Vector 等都繼承自該類型

```python
Tensor.arange(0..9) # Tensor [10]
```

* reshape(self, NewShape: [Nat; M]) -> Self NewShape

```python
(1..9).into(Tensor).reshape [3, 3]
```

* identity i: Nat -> Self shape: [Nat; N]
* zeros(Shape: [Nat; N]) -> Self
* ones(Shape: [Nat; N]) -> Self

* diag

* linspace
* logspace
* geomspace
