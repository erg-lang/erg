# Nat

自然数を表す型。配列のインデックスや範囲型などで使われる。

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
