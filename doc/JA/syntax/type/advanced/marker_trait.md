# Marker Trait


マーカートレイトは、要求属性のないトレイトである。すなわち、メソッドを実装せずにImplすることができる。
要求属性がないと意味がないように思えるが、そのトレイトに属しているという情報が登録されるので、パッチメソッドを使ったり、コンパイラが特別扱いしたりできる。

すべてのマーカートレイトは`Marker`トレイトに包摂される。
標準で提供されている`Light`はマーカートレイトの一種である。

```python
Light = Subsume Marker
```

```python
Person = Class {.name = Str; .age = Nat} and Light
```

```python
M = Subsume Marker

MarkedInt = Inherit Int, Impl := M

i = MarkedInt.new(2)
assert i + 1 == 2
assert i in M
```

マーカークラスは`Excluding`引数で外すことも可能である。

```python
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```
