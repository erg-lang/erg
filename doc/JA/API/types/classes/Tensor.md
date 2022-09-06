# Tensor Shape: [Nat; N]

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Tensor.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Tensor.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

  多次元配列を効率的に操作するためのクラス。多次元配列に対する積などの演算も定義する。
  Matrix, Vectorなどはこの型を継承している。

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
