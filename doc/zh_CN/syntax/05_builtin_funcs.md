# 内置函数

＃＃ 如果

`if` 是一个根据条件改变处理的函数。

```python
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

`.sample!()` 返回一组随机值。 如果返回值为真，`print! “真”`被执行。
如果条件为假，您还可以指定要执行的操作； 第二个 do 块称为 else 块。

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

如果进程是单行，则可以省略缩进。

```python
result = if Bool.sample!():
    do 1
    do 0
```

## for

你可以使用 `for` 来编写一个重复的过程。

```python
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>上一页</a> | <a href='./06_operator.md'>下一页</a>
</p>
