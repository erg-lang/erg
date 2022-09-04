# Add R

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
f: |O: Type; A <: Add(Int, O)| A -> O
f x = x + 1

g: |A, O: Type; Int <: Add(A, O)| A -> O
g x = 1 + x
```
