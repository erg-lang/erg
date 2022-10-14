# Option T = T or NoneType

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Option.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Option.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

表示"可能失敗"的類型

## 方法

* unwrap(self, msg = "unwrapped a None value") -> T or Panic

提取它，期望內容是 `T` 類型。 如果是 `None`，則輸出 `msg` 并恐慌

```python
x = "...".parse(Int).into(Option Int)
x.unwrap() # UnwrappingError: unwrapped a None value
x.unwrap("failed to convert from string to number") # UnwrappingError: failed to convert from string to number
```

* unwrap_or(self, else: T) -> T

* unwrap_or_exec(self, f: () -> T) -> T

* unwrap_or_exec!(self, p!: () => T) -> T
