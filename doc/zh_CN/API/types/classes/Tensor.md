# Tensor Shape: [Nat; N]

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Tensor.md%26commit_hash%3Db07c17708b9141bbce788d2e5b3ad4f365d342fa)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Tensor.md&commit_hash=b07c17708b9141bbce788d2e5b3ad4f365d342fa)

  用于有效操作多维数组的类。它还定义了诸如多维数组上的乘法之类的操作
  Matrix、Vector 等都继承自该类型

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
