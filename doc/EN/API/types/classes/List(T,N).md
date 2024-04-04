# List T: Type, N: Nat

`[T; N]` is syntactic sugar. `N` can be emitted (`[T; _]`).

## methods

* values_at(self, selectors: [Nat; N]) -> [T; N]

```python
assert ["a", "b", "c", "d", "e"].values_at([0, 1, 3]) == ["a", "b", "d"]
```

* all(self, pred: T -> Bool) -> Bool
   Returns whether all elements satisfy pred.
   If the element is 0, it will be `True` regardless of pred, but a Warning will be issued.
   This specification itself has been adopted by many languages and is required for logical consistency.

   ```python
   assert[].all(_ -> False)
   ```

   ```python
   assert all(False for _in[])
   ```

## methods of List T, N | T <: Eq

* freq self -> [{T: Nat}]
   Returns the frequency of occurrence of an object.

```python
assert ["a", "b", "c", "b", "c", "b"].freq() \
== [{"a", 1}, {"b": 3}, {"c": 2}]
```
