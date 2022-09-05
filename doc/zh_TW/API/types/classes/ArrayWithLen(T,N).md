# ArrayWithLen T: Type, N: Nat

`[T; N]`是語法糖。還有一個[`Array` 類型](./Array.md)省略了長度。

## 方法

* values_at(self, selectors: [Nat; N]) -> [T; N]

```python
assert ["a", "b", "c", "d", "e"].values_at([0, 1, 3]) == ["a", "b", "d"]
```

* all(self, pred: T -> Bool) -> Bool
  返回是否所有元素都滿足 pred。
   如果元素為 0，則無論 pred 為 `True`，但會發出警告。
   該規范本身已被多種語言采用，是邏輯一致性所必需的。

  ```python
  assert [].all(_ -> False)
  ```

  ```python
  assert all(False for _ in [])
  ```

## ArrayWithLen T, N | T <: Eq 的方法

* freq self -> [{T: Nat}]
  返回對象出現的次數。

```python
assert ["a", "b", "c", "b", "c", "b"].freq() \
== [{"a", 1}, {"b": 3}, {"c": 2}]
```
