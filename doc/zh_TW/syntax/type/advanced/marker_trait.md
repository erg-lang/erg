# 標記特征

標記特征是沒有必需屬性的特征。 也就是說，您可以在不實現任何方法的情況下實現 Impl。
沒有 required 屬性似乎沒有意義，但由于注冊了它屬于 trait 的信息，因此可以使用 patch 方法或由編譯器進行特殊處理。

所有標記特征都包含在“標記”特征中。
作為標準提供的“光”是一種標記特征。

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

標記類也可以使用 `Excluding` 參數排除。

```python
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```