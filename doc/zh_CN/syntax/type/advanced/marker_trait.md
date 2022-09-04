# Marker Trait

标记托盘是没有要求属性的托盘。也就是说，不安装方法就可以 Impl。如果没有要求属性的话，似乎就没有意义了，但是因为登录了属于该trait的信息，所以可以使用补丁方法，编译器进行特别处理。

所有标记块都包含在块中。标准中提供的<gtr=“6”/>是一种标记托盘。


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

也可以用自变量来排除标记类。


```erg
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```
