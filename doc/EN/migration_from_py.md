# Tips for migrating from Python to Erg

## I want to convert a string to int etc.

Use the `parse` method of the `Str` class. It returns a `Result` type.

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

You can also use the `try_from` method.

```python
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```