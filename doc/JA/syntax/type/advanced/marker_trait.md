# Marker Trait

マーカートレイトは、要求属性のないトレイトである。すなわち、メソッドを実装せずにImplすることができる。
要求属性がないと意味がないように思えるが、そのトレイトに属しているという情報が登録されるので、パッチメソッドを使ったり、コンパイラが特別扱いしたりできる。

すべてのマーカートレイトは`Marker`トレイトに包摂される。
標準で提供されている`Light`はマーカートレイトの一種である。

```erg
Light = Subsume Marker
```

```erg
Person = Class {.name = Str; .age = Nat} and Light
```

```erg
M = Subsume Marker

MarkedInt = Inherit Int, Impl := M

i = MarkedInt.new(2)
assert i + 1 == 2
assert i in M
```

マーカークラスは`Excluding`引数で外すことも可能である。

```erg
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```
