# 从 Python 迁移到 Erg 的提示

## 我想将字符串转换为 int 等。

使用 `Str` 类的 `parse` 方法。 它返回一个 `Result` 类型。

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

您还可以使用 `try_from` 方法。

```python
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```