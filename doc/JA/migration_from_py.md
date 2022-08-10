# PythonからErgへの移行に関してのTips

## 文字列をint等に変換したい

`Str`クラスの`parse`メソッドを使用してください。これは`Result`型を返します。

```python
s: str
i: int = int(s)
```

```erg
s: Str
res: Result(Int, IntParseError) = s.parse Int
i: Int = res.unwrap()
f: Result(Float, FloatParseError) = s.parse Float
```

`try_from`メソッドも使えます。

```erg
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```
