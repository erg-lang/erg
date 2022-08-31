# Tips on migrating from Python to Erg

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/migration_from_py.md%26commit_hash%3D020fa47edd39b86ed44bd8c46822aad6edf1442a)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/migration_from_py.md&commit_hash=020fa47edd39b86ed44bd8c46822aad6edf1442a)

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
