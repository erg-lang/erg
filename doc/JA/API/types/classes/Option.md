# Option T = T or NoneType

「失敗するかもしれない」を表す型。

## methods

* unwrap(self, msg = "unwrapped a None value") -> T or Panic

中身が`T`型であると期待して取り出す。`None`であった場合`msg`を出力してパニックする。

```erg
x = "...".parse(Int).into(Option Int)
x.unwrap() # UnwrappingError: unwrapped a None value
x.unwrap("failed to convert from string to number") # UnwrappingError: failed to convert from string to number
```

* unwrap_or(self, else: T) -> T

* unwrap_or_exec(self, f: () -> T) -> T

* unwrap_or_exec!(self, p!: () => T) -> T
