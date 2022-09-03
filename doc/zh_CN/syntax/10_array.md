# 排列

数组是最基本的。集合是可以在内部包含多个对象的对象。


```erg
a = [1, 2, 3]
a: [Int; 3] # 型指定: セミコロンの後の数字は要素数
# 要素数がわからない場合は省略可能
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

通常，数组不能包含不同类型的对象。


```erg
[1, "a"] # TypeError: 1st element is Int, but 2nd element is Str
```

但是，这种显式类型可以避免限制。


```erg
[1, "a"]: [Int or Str]
```

## 切片

数组还可以同时检索多个值。我们管这个叫切片。


```erg
l = [1, 2, 3, 4]
# Pythonのl[1:3]に相当
assert l[1..<3] == [2, 3]
assert l[1..2] == [2, 3]
# l[1]と同じ
assert l[1..1] == [2]
# Pythonのl[::2]に相当
assert l[..].step(2) == [2, 4]
```

切片获得的对象是数组的（不可变）引用。


```erg
print! Typeof l[1..2] # Ref [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>Previous</a> | <a href='./11_tuple.md'>Next</a>
</p>
