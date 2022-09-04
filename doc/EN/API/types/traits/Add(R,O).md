# Add R

```python
Add R = Trait {
     .AddO = Type
     .`_+_` = (Self, R) -> Self.AddO
}
```

`Add` is a type that defines addition. There are two types of `+` as addition: methods and functions.
`+` as a binary function, i.e. `_+_`, is defined as follows.

```python
`_+_`(l: Add(R, O), r: R): O = l.`_+_` r
```

The purpose of this definition is so that `+` can be treated as a function instead of a method.

```python
assert [1, 2, 3].fold(0, `_+_`) == 6

call op, x, y = op(x, y)
assert call(`_+_`, 1, 2) == 3
```

Addition is typed like this.

```python
f: |O: Type; A <: Add(Int, O)| A -> O
f x = x + 1

g: |A, O: Type; Int <: Add(A, O)| A -> O
g x = 1 + x
```