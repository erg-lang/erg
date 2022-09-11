# 标记特征

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/marker_trait.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/marker_trait.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

标记特征是没有必需属性的特征。 也就是说，您可以在不实现任何方法的情况下实现 Impl。
没有 required 属性似乎没有意义，但由于注册了它属于 trait 的信息，因此可以使用 patch 方法或由编译器进行特殊处理。

所有标记特征都包含在"标记"特征中。
作为标准提供的"光"是一种标记特征。

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

标记类也可以使用 `Excluding` 参数排除。

```python
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```