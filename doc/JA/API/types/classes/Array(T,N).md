# Array T: Type, N: Nat

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Array(T,N).md%26commit_hash%3D13f2d31aee9012f60b7a40d4b764921f1419cdfe)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Array(T,N).md&commit_hash=13f2d31aee9012f60b7a40d4b764921f1419cdfe)

`[T; N]`は糖衣構文。

## methods

* values_at(self, selectors: [Nat; N]) -> [T; N]

```python
assert ["a", "b", "c", "d", "e"].values_at([0, 1, 3]) == ["a", "b", "d"]
```

* all(self, pred: T -> Bool) -> Bool
  全ての要素がpredを満たすかどうかを返す。
  要素が0のときはpredに関わらず`True`となるが、Warningを出す。
  この仕様自体は多くの言語で採用されており、論理学的な整合性から要請される。

  ```python
  assert [].all(_ -> False)
  ```

  ```python
  assert all(False for _ in [])
  ```

## methods of ArrayWithLen T, N | T <: Eq

* freq self -> [{T: Nat}]
  オブジェクトの出現頻度を返す。

```python
assert ["a", "b", "c", "b", "c", "b"].freq() \
== [{"a", 1}, {"b": 3}, {"c": 2}]
```
