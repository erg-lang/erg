# 集

集代表一个集合，在数据结构上是重复的、没有顺序的数组。


```erg
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # 重複は自動で削除される
assert {1, 2} == {2, 1}
```

集合可以进行集合运算。


```erg
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

布景是一个等质的收藏。要使不同类中的对象共存，必须使它们相等。


```erg
s: {Int or Str} = {"a", 1, "b", -1}
```

## 设置为类型

套也可以当作一种类型。这些类型称为。


```erg
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

集的元素将变为类型元素。需要注意的是，布景本身是不一样的。


```erg
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>Previous</a> | <a href='./15_type.md'>Next</a>
</p>
