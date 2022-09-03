# 内置函数

## if

是一个函数，它可以根据条件改变操作。


```erg
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

随机返回集合的值。如果返回值为 true，则执行<gtr=“7”/>。还可以指定当条件为假时如何处理。第二个 do 块称为 else 块。


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

如果只执行一行操作，则可以省略缩进。


```erg
result = if Bool.sample!():
    do 1
    do 0
```

## for

你可以使用来编写重复的操作。


```erg
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>Previous</a> | <a href='./06_operator.md'>Next</a>
</p>
