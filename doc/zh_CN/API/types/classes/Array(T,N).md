# Array T: Type, N: Nat

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Array(T,N).md%26commit_hash%3D13f2d31aee9012f60b7a40d4b764921f1419cdfe)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Array(T,N).md&commit_hash=13f2d31aee9012f60b7a40d4b764921f1419cdfe)

`[T; N]` 是语法法糖. `N` 可以发出 (`[T; _]`).

## methods

* values_at(self, selectors: [Nat; N]) -> [T; N]

```python
assert ["a", "b", "c", "d", "e"].values_at([0, 1, 3]) == ["a", "b", "d"]
```

* all(self, pred: T -> Bool) -> Bool
  返回所有元素是否满足pred。
  如果元素为0，则无论pred如何，它都将为`True`，但将发出警告。
  该规范本身已被许多语言采用，并且是逻辑一致性所必需的。

   ```python
   assert[].all(_ -> False)
   ```

   ```python
   assert all(False for _in[])
   ```

## methods of Array T, N | T <: Eq

* freq self -> [{T: Nat}]
   返回对象的出现频率

```python
assert ["a", "b", "c", "b", "c", "b"].freq() \
== [{"a", 1}, {"b": 3}, {"c": 2}]
```
