# Add R

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Add(R,O).md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Add(R,O).md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

```python
Add R = Trait {
    .AddO = Type
    .`_+_` = (Self, R) -> Self.AddO
}
```

`Add`是一種定義加法的類型。加法有兩種類型的`+`: 方法和函數
`+`作為二元函數，即`_+_`，定義如下: 

```python
`_+_`(l: Add(R, O), r: R): O = l.`_+_` r
```

這個定義的目的是讓 `+` 可以被視為一個函數而不是一個方法

```python
assert [1, 2, 3].fold(0, `_+_`) == 6

call op, x, y = op(x, y)
assert call(`_+_`, 1, 2) == 3
```

加法是這樣輸入的

```python
f: |O: Type; A <: Add(Int, O)| A -> O
f x = x + 1

g: |A, O: Type; Int <: Add(A, O)| A -> O
g x = 1 + x
```
