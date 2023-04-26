# 特殊形式

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/special.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/special.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

特殊形式は、Ergの型システムでは表現ができない演算子、サブルーチン(のようなもの)である。``で囲っているが、実際は捕捉できない。
また、`Pattern`や`Body`, `Conv`といった型が便宜上登場するが、そのような型が存在するわけではない。その意味もコンテクストによって異なる。

## `=`(pat: Pattern, body: Body) -> NoneType

bodyをpatに変数として代入する。同じスコープにすでに変数が存在する場合と、patにマッチしなかった場合にエラーを送出する。
また、レコードの属性定義やデフォルト引数にも使われる。

```python
record = {i = 1; j = 2}
f(x: Int, y = 2) = ...
```

bodyが型か関数であるときに`=`は特殊な振る舞いをする。
左辺の変数名を右辺のオブジェクトに埋め込むのである。

```python
print! Class() # <class <lambda>>
print! x: Int -> x + 1 # <function <lambda>>
C = Class()
print! c # <class C>
f = x: Int -> x + 1
print! f # <function f>
g x: Int = x + 1
print! g # <function g>
K X: Int = Class(...)
print! K # <kind K>
L = X: Int -> Class(...)
print! L # <kind L>
```

`=`演算子は、戻り値が「未定義」である。
多重代入、関数中での`=`は文法エラーとなる。

```python
i = j = 1 # SyntaxError: multiple assignments are not allowed
print!(x=1) # SyntaxError: cannot use `=` in function arguments
# hint: did you mean keyword arguments (`x: 1`)?
if True, do:
    i = 0 # SyntaxError: A block cannot be terminated by an assignment expression
```

## `->`(pat: Pattern, body: Body) -> Func

無名関数、関数型を生成する。

## `=>`(pat: Pattern, body: Body) -> Proc

無名プロシージャ、プロシージャ型を生成する。

## `.`(obj, attr)

objの属性を読み込む。

## `|>`(obj, c: Callable)

`c(obj)`を実行する。`x + y |>.foo()`は`(x + y).foo()`と同じ。

### (x: Option T)`?` -> T

後置演算子。`x.unwrap()`を呼び出し、エラーの場合はその場で`return`する。

## `:`(x, T)

オブジェクト`x`が`T`型であることを宣言する。`x`の型が`T`の部分型でなければエラーとなる。

## `as`(x, T)

オブジェクト`x`を`T`型に強制キャストする。`x`の型が`T`の部分型でなければエラーとなる。
`:`との違いとして、`x: U; U <: T`であるとき`(x: T): U`となるが、`(x as T): T`である。

## match(obj, *arms: Lambda)

objについて、パターンにマッチしたarmを実行する。armは無名関数でなくてはならない。

```python
match [1, 2, 3]:
  (l: Int) -> log "this is type of Int"
  [[a], b] -> log a, b
  [*a] -> log a
# (1, 2, 3)
```

型指定によって処理を分岐できるが、型推論の結果は分岐に影響しない。

```python
zero: {0} -> {0}
one: {1} -> {1}
_ = match x:
    i -> zero i
    j -> one j # Warning: cannot reach this arm
```

## Del(*x: T) -> NoneType

変数`x`を削除する。ただし組み込みのオブジェクトは削除できない。

```python
a = 1
Del a # OK

Del True # SyntaxError: cannot delete a built-in object
```

## do(body: Body) -> Func

引数なしの無名関数を生成する。`() ->`の糖衣構文。

## do!(body: Body) -> Proc

引数なしの無名プロシージャを生成する。`() =>`の糖衣構文。

## 集合演算子

### `[]`(*objs)

引数から配列、またはオプション引数からディクトを生成する。

### `{}`(*objs)

引数からセットを生成する。

### `{}`(*fields: ((Field, Value); N))

レコードを生成する。

### `{}`(layout, *names, *preds)

篩型を生成する。

### `*`

入れ子になったコレクションを展開する。パターンマッチでも使える。

```python
[x, *y] = [1, 2, 3]
assert x == 1 and y == [2, 3]
assert [x, *y] == [1, 2, 3]
assert [*y, x] == [2, 3, 1]
{x; *yz} = {x = 1; y = 2; z = 3}
assert x == 1 and yz == {y = 2; z = 3}
assert {x; *yz} == {x = 1; y = 2; z = 3}
```

## 仮想演算子

ユーザーが直接使用できない演算子です。

### ref(x: T) -> Ref T

オブジェクトの不変参照を返す。

### ref!(x: T!) -> Ref! T!

可変オブジェクトの可変参照を返す。
