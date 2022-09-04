# Marker Trait

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/marker_trait.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/marker_trait.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

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
