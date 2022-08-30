# Built-in functions

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/05_builtin_funcs.md%26commit_hash%3D6dc8c5015b6120497a26d80eaef65d23eb2bee2a)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/05_buildtin_funcs.md&commit_hash=6dc8c5015b6120497a26d80eaef65d23eb2bee2a)

## if

`if` is a function that changes processing depending on a condition.

```erg
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

`.sample!()` returns a random set of values. If the return value is true, `print! "True"` is executed.
You can also specify what to do if the condition is false; the second do block is called the else block.

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

If the process is a single line, you can omit indentation.

```erg
result = if Bool.sample!():
    do 1
    do 0
```

## for

You can use `for` to write a repeating process.

```erg
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>Previous</a> | <a href='./06_operator.md'>Next</a>
</p>
