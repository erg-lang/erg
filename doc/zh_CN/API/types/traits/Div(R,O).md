# Div R, O

如果除以零没有错误，请使用“SafeDiv”

```python
Div R, O = Trait {
    .`/` = Self.(R) -> O or Panic
}
```
