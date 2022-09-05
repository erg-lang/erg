# 從 Python 遷移到 Erg 的提示

## 我想將字符串轉換為 int 等。

使用 `Str` 類的 `parse` 方法。 它返回一個 `Result` 類型。

```python
s: str
i: int = int(s)
```

```python
s: Str
res: Result(Int, IntParseError) = s. parse Int
i: Int = res.unwrap()
f: Result(Float, FloatParseError) = s. parse Float
```

您還可以使用 `try_from` 方法。

```python
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```