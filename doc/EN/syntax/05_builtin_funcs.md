# Built-in functions

## if

`if` is a function that changes processing depending on a condition.

```python
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

`.sample!()` returns a random set of values. If the return value is true, `print! "True"` is executed.
You can also specify what to do if the condition is false; the second do block is called the else block.

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

If the process is a single line, you can omit indentation.

```python
result = if Bool.sample!():
    do 1
    do 0
```

## for

You can use `for` to write a repeating process.

```python
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>Previous</a> | <a href='./06_operator.md'>Next</a>
</p>
