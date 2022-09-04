# 集

集代表一個集合，在數據結構上是重複的、沒有順序的數組。


```erg
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # 重複は自動で削除される
assert {1, 2} == {2, 1}
```

集合可以進行集合運算。


```erg
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

佈景是一個等質的收藏。要使不同類中的對象共存，必須使它們相等。


```erg
s: {Int or Str} = {"a", 1, "b", -1}
```

## 設置為類型

套也可以當作一種類型。這些類型稱為。


```erg
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

集的元素將變為類型元素。需要注意的是，佈景本身是不一樣的。


```erg
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>Previous</a> | <a href='./15_type.md'>Next</a>
</p>