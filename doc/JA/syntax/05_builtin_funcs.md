# 組み込み関数

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/05_builtin_funcs.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/05_builtin_funcs.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

## if

`if`は条件に応じて処理を変える関数です。

```erg
result: Option Int = if! Bool.sample!(), do:
    log "True was chosen"
    1
print! result # None (or 1)
```

`.sample!()`は集合の値をランダムに返します。もし戻り値が真ならば、`print! "True"`が実行されます。
条件が偽であった際の処理も指定できます。２つ目のdoブロックはelseブロックと呼ばれます。

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

処理が1行ならば、インデントを省略できます。

```erg
result = if Bool.sample!():
    do 1
    do 0
```

## for

繰り返し行う処理を書くときは`for`が使えます。

```erg
match_s(ss: Iterator(Str), pat: Pattern): Option Str =
    for ss, s ->
        if pat.match(s).is_some():
            break s
```

<p align='center'>
    <a href='./04_function.md'>Previous</a> | <a href='./06_operator.md'>Next</a>
</p>
