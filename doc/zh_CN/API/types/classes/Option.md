# Option T = T or NoneType

表示“可能失败”的类型。

## methods

* unwrap(self, msg = "unwrapped a None value") -> T or Panic

提取它，期望内容是 `T` 类型。 如果是 `None`，则输出 `msg` 并恐慌

```erg
x = "...".parse(Int).into(Option Int)
x.unwrap() # UnwrappingError: unwrapped a None value
x.unwrap("failed to convert from string to number") # UnwrappingError: failed to convert from string to number
```

* unwrap_or(self, else: T) -> T

* unwrap_or_exec(self, f: () -> T) -> T

* unwrap_or_exec!(self, p!: () => T) -> T
