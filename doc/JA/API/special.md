
# 特殊形式(Special form)

特殊形式は、Ergの型システムでは表現ができない演算子、サブルーチン(のようなもの)である。``で囲っているが、実際は捕捉できない。
また、`Pattern`や`Body`, `Conv`といった型が便宜上登場するが、そのような型が存在するわけではない。その意味もコンテクストによって異なる。

## `=`(pat: Pattern, body: Body) -> NoneType

bodyをpatに変数として代入する。同じスコープにすでに変数が存在する場合と、patにマッチしなかった場合にエラーを送出する。
また、レコードの属性定義やデフォルト引数にも使われる。

```erg
record = {i = 1; j = 2}
f(x: Int, y = 2) = ...
```

bodyが型か関数であるときに`=`は特殊な振る舞いをする。
左辺の変数名を右辺のオブジェクトに埋め込むのである。

```erg
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

```erg
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

## `:`(subject, T)

subjectがTに合致しているか判定する。合致していない場合はコンパイルエラーを送出する。

```erg
a: Int
f x: Int, y: Int = x / y
```

また、`:`適用スタイルにも使われる。

```erg
f x:
    y
    z
```

`:`も`=`と同じく演算の結果が未定義である。

```erg
_ = x: Int # SyntaxError:
print!(x: Int) # SyntaxError:
```

## `.`(obj, attr)

objの属性を読み込む。
`x.[y, z]`とすると、xのyとzという属性を配列にして返す。

## `|>`(obj, c: Callable)

`c(obj)`を実行する。`x + y |>.foo()`は`(x + y).foo()`と同じ。

### (x: Option T)`?` -> T | T

後置演算子。`x.unwrap()`を呼び出し、エラーの場合はその場で`return`する。

## match(obj, ...lambdas: Lambda)

objについて、パターンにマッチしたlambdaを実行する。

```erg
match [1, 2, 3]:
  (l: Int) -> log "this is type of Int"
  [[a], b] -> log a, b
  [...a] -> log a
# (1, 2, 3)
```

## del(x: ...T) -> NoneType | T

変数`x`を削除する。ただし組み込みのオブジェクトは削除できない。

```erg
a = 1
del a # OK

del True # SyntaxError: cannot delete a built-in object
```

## do(body: Body) -> Func

引数なしの無名関数を生成する。`() ->`の糖衣構文。

## do!(body: Body) -> Proc

引数なしの無名プロシージャを生成する。`() =>`の糖衣構文。

## `else`(l, r) -> Choice

Choiceオブジェクトという２つ組のタプルのような構造体を生成する。
`l, r`は遅延評価される。すなわち、`.get_then`または`.get_else`が呼ばれたとき初めて式が評価される。

```erg
choice = 1 else 2
assert choice.get_then() == 1
assert choice.get_else() == 2
assert True.then(choice) == 1
```

## 集合演算子

### `[]`(...objs)

引数から配列、またはオプション引数からディクトを生成する。

### `{}`(...objs)

引数からセットを生成する。

### `{}`(...fields: ((Field, Value); N))

レコードを生成する。

### `{}`(layout, ...names, ...preds)

篩型、ランク2型を生成する。

### `...`

入れ子になったコレクションを展開する。パターンマッチでも使える。

```erg
[x, ...y] = [1, 2, 3]
assert x == 1 and y == [2, 3]
assert [x, ...y] == [1, 2, 3]
assert [...y, x] == [2, 3, 1]
{x; ...yz} = {x = 1; y = 2; z = 3}
assert x == 1 and yz == {y = 2; z = 3}
assert {x; ...yz} == {x = 1; y = 2; z = 3}
```

## 仮想演算子

ユーザーが直接使用できない演算子です。

### ref(x: T) -> Ref T | T

オブジェクトの不変参照を返す。

### ref!(x: T!) -> Ref! T! | T!

可変オブジェクトの可変参照を返す。
