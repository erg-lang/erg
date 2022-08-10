# Tips on migrating from Python to Erg

## Want to convert a string to an int, etc

Use the `parse` method of the `Str` class. It returns a `Result` type.

```python
# Python
s: str
i: int = int(s)
```

```erg
# Erg
s: Str
res: Result(Int, IntParseError) = s.parse Int
i: Int = res.unwrap()
f: Result(Float, FloatParseError) = s.parse Float
```

You can also use the `try_from` method.

```erg
# Erg
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```
