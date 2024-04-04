# 量化依存型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/quantified_dependent.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/quantified_dependent.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

Ergには量化型、依存型が存在します。すると当然、その二つを組み合わせた型を作ることができます。それが量化依存型です。

```python
NonNullStr = |N: Nat| StrWithLen N | N != 0 # N: Nat; S: StrWithLen N; N != 0}と同じ
NonEmptyList = |N: Nat| [_; N | N > 0] # N: Nat; A: List(_, N); N > 0}と同じ
```

量化依存型の標準形は`K(A, ... | Pred)`です。`K`は型構築子、`A, B`は型引数、`Pred`は条件式です。

左辺値としての量化依存型は、元の型と同じモジュール内でのみメソッドを定義出来ます。

```python
K A: Nat = Class ...
K(A).
    ...
K(A | A >= 1).
    method ref! self(A ~> A+1) = ...
```

右辺値としての量化依存型は、使用する型変数を型変数リスト(`||`)で宣言する必要がある。

```python
# Tは具体的な型
a: |N: Nat| [T; N | N > 1]
```
