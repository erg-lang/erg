# 內置函數

## if

是一個函數，它可以根據條件改變操作。


```erg
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

隨機返回集合的值。如果返回值為 true，則執行<gtr=“7”/>。還可以指定當條件為假時如何處理。第二個 do 塊稱為 else 塊。


```erg
result: Nat = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
print! result # 1 (or 0)
```

如果只執行一行操作，則可以省略縮進。


```erg
result = if Bool.sample!():
    do 1
    do 0
```

## for

你可以使用來編寫重複的操作。


```erg
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>Previous</a> | <a href='./06_operator.md'>Next</a>
</p>