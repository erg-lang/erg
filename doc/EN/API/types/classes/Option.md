# Option T = T or NoneType

A type that represents "may fail".

## methods

* unwrap(self, msg = "unwrapped a None value") -> T or Panic

Extract it expecting the contents to be `T` type. If it is `None`, output `msg` and panic.

```python
x = "...".parse(Int).into(Option Int)
x.unwrap() # UnwrappingError: unwrapped a None value
x.unwrap("failed to convert from string to number") # UnwrappingError: failed to convert from string to number
```

* unwrap_or(self, else: T) -> T

* unwrap_or_exec(self, f: () -> T) -> T

* unwrap_or_exec!(self, p!: () => T) -> T