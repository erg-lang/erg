# Python 到 Erg 遷移的 Tips

## 要將字符串轉換為 int 等

請使用類中的<gtr=“5”/>方法。它返回類型<gtr=“6”/>。


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

也可以使用方法。


```erg
s: Str
i: Int = Int.try_from(s).unwrap()
f: Float = Float.try_from(s).unwrap()
```