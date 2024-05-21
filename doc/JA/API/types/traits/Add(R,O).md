# Add R

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Add(R,O).md%26commit_hash%3Df4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Add(R,O).md&commit_hash=f4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)

```python
Add R = Trait {
    .AddO = Type
    .`_+_` = (Self, R) -> Self.AddO
}
```

`Add`は加算を定義する型である。加算としての`+`にはメソッドと関数の2種類がある。
二項関数としての`+`、すなわち`_+_`は、以下のように定義されている。

```python
`_+_`(l: Add(R, O), r: R): O = l.`_+_` r
```

わざわざこの定義があるのは、`+`をメソッドではなく関数として取り扱えるようにである。

```python
assert [1, 2, 3].fold(0, `_+_`) == 6

call op, x, y = op(x, y)
assert call(`_+_`, 1, 2) == 3
```

加算はこのように型付けされる。

```python
f: |O: Type, A <: Add(Int, O)| A -> O
f x = x + 1

g: |A, O: Type, Int <: Add(A, O)| A -> O
g x = 1 + x
```
