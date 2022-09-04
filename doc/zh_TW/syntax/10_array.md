# 排列

數組是最基本的。集合是可以在內部包含多個對象的對象。


```erg
a = [1, 2, 3]
a: [Int; 3] # 型指定: セミコロンの後の數字は要素數
# 要素數がわからない場合は省略可能
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

通常，數組不能包含不同類型的對象。


```erg
[1, "a"] # TypeError: 1st element is Int, but 2nd element is Str
```

但是，這種顯式類型可以避免限制。


```erg
[1, "a"]: [Int or Str]
```

## 切片

數組還可以同時檢索多個值。我們管這個叫切片。


```erg
l = [1, 2, 3, 4]
# Pythonのl[1:3]に相當
assert l[1..<3] == [2, 3]
assert l[1..2] == [2, 3]
# l[1]と同じ
assert l[1..1] == [2]
# Pythonのl[::2]に相當
assert l[..].step(2) == [2, 4]
```

切片獲得的對像是數組的（不可變）引用。


```erg
print! Typeof l[1..2] # Ref [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>Previous</a> | <a href='./11_tuple.md'>Next</a>
</p>