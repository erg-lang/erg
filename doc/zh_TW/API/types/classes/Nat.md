# Nat

表示自然數的類型。用於數組索引和範圍類型

## def

```erg
Nat = 0.._
```

## methods

* times!(self, p: () => NoneType) -> NoneType

```erg
100.times! () =>
    print! "hello!"
```