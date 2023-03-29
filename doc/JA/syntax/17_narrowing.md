# 型の絞り込み

Ergでは条件分岐による型の絞り込み(narrowing)ができます。

```python,compile_fail
x: Int or NoneType
y = x + 1 # TypeError
```

`x`の型は`Int or NoneType`です。`None`である可能性があるため、`x + 1`は型エラーになります。

```python
if x != None, do:
    x + 1 # OK
    ...
```

しかし、上のように`x`が`None`でないことを条件分岐で確認することで、`x`の型は`Int`に絞り込まれます。
`isinstance`演算子でも同様のことができます。

```python
if isinstance(x, Int), do:
    x + 1 # OK
    ...
```

## 絞り込み効果を発生させる関数・演算子

現在のところ、絞り込みの効果を発生させる関数・演算子は以下のものに限られます。

### `in`

`x in T`という式は、`x`が`T`のインスタンスであるかどうかを判定します。
これが`True`となった場合、絞り込みを使うサブルーチンで効果を発揮します。

```python
x: Int or Str
if x in Int, do:
    x + 1 # OK
    ...
```

### `notin`

`in`と逆の意味を持ちます。

### `isinstance`

`x in T`と似ていますが、型が単純なクラスである場合にのみ使えます。

```python
x in 1.. # OK
isinstance(x, 1..) # TypeError
isinstance(x, Int) # OK
```

### `==`/`is!`

`x == y`ないし`x is! y`という式は、`x`が`y`に等しいかどうかを判定します(両者の違いはAPIのドキュメント等を参照してください)。

### `!=`/`isnot!`

`==`/`is!`と逆の意味を持ちます。

### `>=`/`>`/`<=`/`<`

絞り込みによって篩型のメソッドが使えるようになる場合があります。

```python
i: Int
if i >= 0, do:
    log i.times! # <bound method ...>
```

## 絞り込み効果を消費する関数・演算子

`if/if!/while!`は引数に渡したブロック内でのみ絞り込みが発生します。
スコープを抜けると絞り込みは解除されます。
`assert`の場合は、`assert`呼び出し以降のブロック内でのみ絞り込みが発生します。

### `if`/`if!`

```python
x: Int or Str
if x in Int, do:
    x + 1 # OK
    ...
```

### `while!`

```python
x: Int! or NoneType
while! do x != None, do!:
    x.inc!() # OK
    ...
```

### `assert`

```python
x: Int or NoneType
assert x != None
x: Int
```

<p align='center'>
    <a href='./16_type.md'>Previous</a> | <a href='./18_iterator.md'>Next</a>
</p>
