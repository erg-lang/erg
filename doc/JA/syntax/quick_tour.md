# Quick Tour

`syntax`以下のドキュメントは、概ねプログラミング初心者でも理解できることを目指して書かれています。
すでにPythonやRust, Haskellなどの言語を習得されている方にとっては、少し冗長であるかもしれません。

そこで以下では概説的にErgの文法を紹介します。
特に言及のない部分はPythonと同じと考えてください。

## 変数、定数

変数は`=`で定義します。Haskellと同じように、一度定義した変数は書き換えられません。ただし別のスコープではシャドーイングできます。

```python
i = 0
if True:
    i = 1
assert i == 0
```

大文字で始まるものは定数です。コンパイル時計算できるものだけが定数にできます。
また、定数は定義以降すべてのスコープで同一です。

```python
PI = 3.141592653589793
match random.random!(0..10):
    PI:
        log "You get PI, it's a miracle!"
```

## 宣言

Pythonと違い、変数の型のみを先に宣言することが可能です。
当然、宣言の型と実際に代入されるオブジェクトの型は互換していなくてはなりません。

```python
i: Int
i = 10
```

## 関数

Haskellと同じように定義できます。

```python
fib 0 = 0
fib 1 = 1
fib n = fib(n - 1) + fib(n - 2)
```

無名関数は以下のように定義できます。

```python
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## 演算子

Erg独自の演算子は以下の通りです。

### 可変化演算子(!)

Ocamlの`ref`のようなものです。

```python
i = !0
i.update! x -> x + 1
assert i == 1
```

## プロシージャ

副作用のあるサブルーチンはプロシージャと呼ばれ、`!`がついています。

```python
print! 1 # 1
```

## ジェネリック関数(多相関数)

```python
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## レコード

ML系言語にあるレコード(あるいはJSのオブジェクトリテラル)に相当するものを利用できます。

```python
p = {x = 1; y = 2}
```

## 所有権

Ergは可変オブジェクト(`!`演算子で可変化したオブジェクト)に所有権がついており、複数の場所から書き換えられません。

```python
i = !0
j = i
assert j == 0
i # MoveError
```

対して不変オブジェクトは複数の場所から参照できます。

## 可視性

変数の頭に`.`をつけると、その変数は公開変数となり、外部モジュールから参照できるようになります。

```python
# foo.er
.x = 1
y = 1
```

```python
foo = import "foo"
assert foo.x == 1
foo.y # VisibilityError
```

## パターンマッチ

### 変数パターン

```python
# basic assignment
i = 1
# with type
i: Int = 1
# function
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### リテラルパターン

```python
# if `i` cannot be determined to be 1 at compile time, TypeError occurs.
# short hand of `_: {1} = i`
1 = i
# simple pattern matching
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# fibonacci function
fib 0 = 0
fib 1 = 1
fib n: Nat = fib n-1 + fib n-2
```

### 定数パターン

```python
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### 破棄(ワイルドカード)パターン

```python
_ = 1
_: Int = 1
right(_, r) = r
```

### 可変長パターン

後述するタプル/配列/レコードパターンと組み合わせて使う。

```python
[i, ...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### タプルパターン

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# ネストしていないなら()を省略可能(1, 2は(1, 2)として扱われる)
m, n = 1, 2
```

### 配列パターン

```python
length [] = 0
length [_, ...rest] = 1 + length rest
```

#### レコードパターン

```python
{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### データクラスパターン

```python
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## 内包表記

```python
odds = [i | i <- 1..100; i % 2 == 0]
```

## クラス

Ergでは多重・多段継承をサポートしていません。

## トレイト

Rustのトレイトと似ていますが、より本来の意味に近いもので、合成や分離ができ、属性とメソッドは対等に扱われます。
また、実装を伴いません。

```python
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## パッチ

クラスやトレイトに実装を与えたりできます。

## 篩型

述語式で型に制限をかけられます。

```python
Nat = {I: Int | I >= 0}
```

## 値を含むパラメトリック型(依存型)

```python
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```
