# ArrayWithLen T: Type, N: Nat

`[T; N]`是語法糖。還有一個[`Array` 類型](./Array.md)省略了長度。

## methods

* values_at(self, selectors: [Nat; N]) -> [T; N]

```erg
assert ["a", "b", "c", "d", "e"].values_at([0, 1, 3]) == ["a", "b", "d"]
```

* all(self, pred: T -> Bool) -> Bool
  返回是否所有元素都滿足 pred。
   如果元素為 0，則無論 pred 為 `True`，但會發出警告。
   該規範本身已被多種語言採用，是邏輯一致性所必需的。

  ```erg
  assert [].all(_ -> False)
  ```

  ```python
  assert all(False for _ in [])
  ```

## methods of ArrayWithLen T, N | T <: Eq

* freq self -> [{T: Nat}]
  返回對像出現的次數。

```erg
assert ["a", "b", "c", "b", "c", "b"].freq() \
== [{"a", 1}, {"b": 3}, {"c": 2}]
```