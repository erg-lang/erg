# Add R

```python
Add R = Trait {
    .AddO = Type
    .`_+_` = (Self, R) -> Self.AddO
}
```

`Add`是一種定義加法的類型。加法有兩種類型的`+`：方法和函數
`+`作為二元函數，即`_+_`，定義如下：

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
