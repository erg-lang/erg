# 関数

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/04_function.md%26commit_hash%3D00c05ab6686062ea6707a326c840d910a55e6dc4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/04_function.md&commit_hash=00c05ab6686062ea6707a326c840d910a55e6dc4)

関数は「引数」を受け取ってそれを加工し、「戻り値」として返すブロックです。以下のように定義します。

```python
add x, y = x + y
# or
add(x, y) = x + y
```

関数定義の際に指定される引数は、詳しくは仮引数(parameter)と呼ばれるものです。
これに対し、関数呼び出しの際に渡される引数は、実引数(argument)と呼ばれるものです。
`add`は`x`と`y`を仮引数として受け取り、それを足したもの、`x + y`を返す関数です。
定義した関数は、以下のようにして呼び出し(適用)ができます。

```python
add 1, 2
# or
add(1, 2)
```

## コロン適用スタイル

関数は`f x, y, ...`のように呼び出しますが、__実引数__ が多く一行では長くなりすぎる場合は`:`(コロン)を使った適用も可能です。

```python
f some_long_name_variable_1 + some_long_name_variable_2, some_long_name_variable_3 * some_long_name_variable_4
```

```python
f some_long_name_variable_1 + some_long_name_variable_2:
    some_long_name_variable_3 * some_long_name_variable_4
```

上の2つのコードは同じ意味です。このスタイルは`if`関数などを使用するときにも便利です。

```python
result = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
```

この場合、`:`の後はコメント以外のコードを書いてはならず、必ず改行しなくてはなりません。
また、関数の直後に`:`を使うことはできません。これができるのは`do`と`do!`のみです。

```python
# NG
f:
    x
    y
```

## キーワード引数

引数の数が多い関数を定義されていると、引数を渡す順番を間違える危険性があります。
そのような場合はキーワード引数を使用して呼び出すと安全です。

```python
f x, y, z, w, v, u: Int = ...
```

上に定義された関数は、引数が多く、分かりにくい並びをしています。
このような関数は作るべきではありませんが、他人の書いたコードを使うときにこのようなコードにあたってしまうかもしれません。
そこで、キーワード引数を使います。キーワード引数は並びよりも名前が優先されるため、順番を間違えていても名前から正しい引数に値が渡されます。

```python
f u: 6, v: 5, w: 4, x: 1, y: 2, z: 3
```

キーワード引数と`:`の後にすぐ改行してしまうとコロン適用スタイルとみなされるので注意してください。

```python
# means `f(x: y)`
f x: y

# means `f(x, y)`
f x:
    y
```

## デフォルト引数

ある引数が大抵の場合決まりきっており省略できるようにしたい場合、デフォルト引数を使うと良いでしょう。

デフォルト引数は`:=`(or-assign operator)で指定します。`base`が指定されなかったら`math.E`を`base`に代入します。

```python
math_log x: Ratio, base := math.E = ...

assert math_log(100, 10) == 2
assert math_log(100) == math_log(100, math.E)
```

引数を指定しないことと`None`を代入することは区別されるので注意してください。

```python
p! x := 0 = print! x
p!(2) # 2
p!() # 0
p!(None) # None
```

型指定、パターンと併用することもできます。

```python
math_log x, base: Ratio := math.E = ...
f [x, y] := [1, 2] = ...
```

しかしデフォルト引数内では、後述するプロシージャを呼び出したり、可変オブジェクトを代入したりすることができません。

```python
f x := p! 1 = ... # NG
```

また、定義したばかりの引数はデフォルト引数に渡す値として使えません。

```python
f x := 1, y := x = ... # NG
```

## 可変長引数

引数をログ(記録)として出力する`log`関数は、任意の個数の引数を受け取ることができます。

```python
log "Hello", "World", "!" # Hello World !
```

このような関数を定義したいときは、引数に`...`を付けます。このようにすると、引数を可変長の配列として受け取ることができます。

```python
f x: ...Int =
    for x, i ->
        log i

# x == [1, 2, 3, 4, 5]
f 1, 2, 3, 4, 5
```

## 複数パターンによる関数定義

```python
fib n: Nat =
    match n:
        0 -> 0
        1 -> 1
        n -> fib(n - 1) + fib(n - 2)
```

上のような定義直下に`match`が現れる関数は、下のように書き直すことができます。

```python
fib 0 = 0
fib 1 = 1
fib(n: Nat): Nat = fib(n - 1) + fib(n - 2)
```

複数のパターンによる関数定義は、いわゆるオーバーロード(多重定義)ではないことに注意してください。1つの関数はあくまで単一の型のみを持ちます。上の例では、`n`は`0`や`1`と同じ型である必要があります。また、`match`と同じくパターンの照合は上から順に行われます。

違うクラスのインスタンスが混在する場合は、最後の定義で関数引数がOr型であることを明示しなくてはなりません。

```python
f "aa" = ...
f 1 = ...
# `f x = ...` is invalid
f x: Int or Str = ...
```

また、`match`と同じく網羅性がなくてはなりません。

```python
fib 0 = 0
fib 1 = 1
# PatternError: pattern of fib's parameter is not exhaustive
```

しかし、上のような場合でも、後述する[篩型](./type/12_refinement.md)を使って明示的に型指定することで、網羅性を獲得できます。

```python
fib: 0..1 -> 0..1
fib 0 = 0
fib 1 = 1
# OK
```

## 再帰関数

再帰関数は自身を定義に含む関数です。

簡単な例として階乗の計算を行う関数`factorial`を定義してみます。階乗とは、「それ以下の正の数をすべてかける」計算です。
5の階乗は`5*4*3*2*1 == 120`となります。

```python
factorial 0 = 1
factorial 1 = 1
factorial(n: Nat): Nat = n * factorial(n - 1)
```

まず階乗の定義から、0と1の階乗はどちらも1です。
順に考えて、2の階乗は`2*1 == 2`、3の階乗は`3*2*1 == 6`、4の階乗は`4*3*2*1 == 24`となります。
ここでよく見ると、ある数nの階乗はその前の数n-1の階乗にnをかけた数となることがわかります。
これをコードに落とし込むと、`n * factorial(n - 1)`となるわけです。
`factorial`の定義に自身が含まれているので、`factorial`は再帰関数です。

注意として、型指定を付けなかった場合はこのように推論されます。

```python
factorial: |T <: Sub(Int, T) and Mul(Int, Int) and Eq(Int)| T -> Int
factorial 0 = 1
factorial 1 = 1
factorial n = n * factorial(n - 1)
```

しかし例え推論が出来たとしても、再帰関数には型を明示的に指定しておくべきです。上の例では、`factorial(-1)`のようなコードは有効ですが、

```python
factorial(-1) == -1 * factorial(-2) == -1 * -2 * factorial(-3) == ...
```

となって、この計算は停止しません。再帰関数は慎重に値の範囲を定義しないと無限ループに陥ってしまう可能性があります。
型指定は想定しない値の受け入れを防ぐのにも役立つというわけです。

## コンパイル時関数

関数名を大文字で始めるとコンパイル時関数となります。ユーザー定義のコンパイル時関数は、引数がすべて定数で、かつ型を明示する必要があります。
コンパイル関数ができることは限られています。コンパイル時関数内で使えるのは定数式のみ、すなわち、いくつかの演算子(四則演算や比較演算、型構築演算など)とコンパイル時関数のみです。代入する引数も定数式である必要があります。
そのかわり、計算をコンパイル時に行うことができるというメリットがあります。

```python
Add(X, Y: Nat): Nat = X + Y
assert Add(1, 2) == 3

Factorial 0 = 1
Factorial(X: Nat): Nat = X * Factorial(X - 1)
assert Factorial(10) == 3628800

math = import "math"
Sin X = math.sin X # ConstantError: this function is not computable at compile time
```

コンパイル時関数は多相型の定義などでもよく使われます。

```python
Option T: Type = T or NoneType
Option: Type -> Type
```

## 付録1: 関数の比較

Ergでは、関数に`==`が定義されていません。それは関数の構造的な同値性判定アルゴリズムが一般には存在しないためです。

```python
f = x: Int -> (x + 1)**2
g = x: Int -> x**2 + 2x + 1

assert f == g # TypeError: cannot compare functions
```

`f`と`g`は常に同じ結果を返しますが、その判定を行うのは至難の業です。コンパイラに代数学を教え込む必要があります。
そのため、Ergは関数の比較をまるごと諦めており、`(x -> x) == (x -> x)`もコンパイルエラーになります。これはPythonとは違った仕様なので注意する必要があります。

```python
# Pythonの奇妙な例
f = lambda x: x
assert f == f # True
assert (lambda x: x) != (lambda x: x) # Passed
```

## 付録2: ()の補完

```python
f x: Object = ...
# これは以下のように補完される
f(x: Object) = ...

f a
# これは以下のように補完される
f(a)

f a, b # TypeError: f() takes 1 positional argument but 2 were given
f(a, b) # TypeError: f() takes 1 positional argument but 2 were given
f((a, b)) # OK
```

関数型`T -> U`は実際のところ、`(T,) -> U`の糖衣構文です。

<p align='center'>
    <a href='./03_declaration.md'>Previous</a> | <a href='./05_builtin_funcs.md'>Next</a>
</p>
