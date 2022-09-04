# Marker Trait

標記托盤是沒有要求屬性的托盤。也就是說，不安裝方法就可以 Impl。如果沒有要求屬性的話，似乎就沒有意義了，但是因為登錄了屬於該trait的信息，所以可以使用補丁方法，編譯器進行特別處理。

所有標記塊都包含在塊中。標準中提供的<gtr=“6”/>是一種標記托盤。


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

也可以用自變量來排除標記類。


```erg
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```