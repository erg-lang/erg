# 內置函數

＃＃ 如果

`if` 是一個根據條件改變處理的函數。

```python
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

`.sample!()` 返回一組隨機值。 如果返回值為真，`print! “真”`被執行。
如果條件為假，您還可以指定要執行的操作； 第二個 do 塊稱為 else 塊。

```python
result: Nat = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
print! result # 1 (or 0)
```

如果進程是單行，則可以省略縮進。

```python
result = if Bool.sample!():
    do 1
    do 0
```

## for

你可以使用 `for` 來編寫一個重復的過程。

```python
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>上一頁</a> | <a href='./06_operator.md'>下一頁</a>
</p>
