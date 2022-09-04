# Div R, O

如果除以零沒有錯誤，請使用“SafeDiv”

```erg
Div R, O = Trait {
    .`/` = Self.(R) -> O or Panic
}
```