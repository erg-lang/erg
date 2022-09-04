# Div R, O

ゼロ除算によるエラーがない場合は`SafeDiv`を使ってください。

```python
Div R, O = Trait {
    .`/` = Self.(R) -> O or Panic
}
```
