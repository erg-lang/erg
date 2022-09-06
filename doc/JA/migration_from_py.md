# PythonからErgへの移行に関してのTips

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/migration_from_py.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/migration_from_py.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## 文字列をint等に変換したい

`Str`クラスの`parse`メソッドを使用してください。これは`Result`型を返します。

```python
s: str
i: int = int(s)
```

```python
s: Str
res: Result(Int, IntParseError) = s.parse Int
i: Int = res.unwrap()
f: Result(Float, FloatParseError) = s.parse Float
```

`try_from`メソッドも使えます。

```python
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```
