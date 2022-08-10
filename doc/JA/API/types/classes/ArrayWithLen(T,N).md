# ArrayWithLen T: Type, N: Nat

`[T; N]`は糖衣構文。長さを省いた[`Array`型](./Array.md)もある。

## methods

* values_at(self, selectors: [Nat; N]) -> [T; N]

```erg
assert ["a", "b", "c", "d", "e"].values_at([0, 1, 3]) == ["a", "b", "d"]
```

* all(self, pred: T -> Bool) -> Bool
  全ての要素がpredを満たすかどうかを返す。
  要素が0のときはpredに関わらず`True`となるが、Warningを出す。
  この仕様自体は多くの言語で採用されており、論理学的な整合性から要請される。

  ```erg
  assert [].all(_ -> False)
  ```

  ```python
  assert all(False for _ in [])
  ```

## methods of ArrayWithLen T, N | T <: Eq

* freq self -> [{T: Nat}]
  オブジェクトの出現頻度を返す。

```erg
assert ["a", "b", "c", "b", "c", "b"].freq() \
== [{"a", 1}, {"b": 3}, {"c": 2}]
```
