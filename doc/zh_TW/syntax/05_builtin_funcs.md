# 內置函數

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/05_builtin_funcs.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/05_builtin_funcs.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## 如果

`if` 是一個根據條件改變處理的函數。

```python
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

`.sample!()` 返回一組隨機值。 如果返回值為真，`print! "真"`被執行。
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
