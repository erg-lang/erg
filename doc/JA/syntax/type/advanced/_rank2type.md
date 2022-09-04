# rank-2多相

> __Warning__: このドキュメントは情報が古く、全般に間違いを含みます。

Ergでは`id|T|(x: T): T = x`などのように色々な型を受け取れる関数、すなわち多相関数を定義できる。
では、多相関数を受け取れる関数は定義できるだろうか？
例えば、このような関数である(この定義は誤りを含むことに注意してほしい)。

```python
# tuple_map(i -> i * 2, (1, "a")) == (2, "aa")になってほしい
tuple_map|T|(f: T -> T, tup: (Int, Str)): (Int, Str) = (f(tup.0), f(tup.1))
```

`1`と`"a"`の型が違うので、無名関数は一度単相化して終わりではないことに注意してほしい。2回単相化する必要がある。
今まで説明してきた型の範疇では、このような関数の定義はできない。型変数にスコープの概念がないからである。
ここで一旦型を離れて、値レベルでのスコープの概念を確認する。

```python
arr = [1, 2, 3]
arr.map i -> i + 1
```

上のコードの`arr`と`i`は違うスコープの変数である。故に、それぞれの生存期間は異なる(`i`の方が短い)。

今までの型は、全ての型変数で生存期間が同一なのである。すなわち、`T`, `X`, `Y`は同時に決定されていて、以降は不変でなければならない。
逆に言えば、`T`を「内側のスコープ」にある型変数とみなすことができるならば`tuple_map`関数を構成できる。そのために用意されたのが、 __ランク2型__ である。

```python
# tuple_map: ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
tuple_map f: (|T: Type| T -> T), tup: (Int, Str) = (f(tup.0), f(tup.1))
assert tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
```

`{(型) | (型変数のリスト)}`という形式の型を全称型といった(詳しくは[全称型](./../quantified.md)を参照)。
いままで見てきた`id`関数は、典型的な全称関数=多相関数である。

```python
id x = x
id: |T: Type| T -> T
```

全称型は関数型構築子`->`との間に特殊な結合の規則を持っており、結合の仕方によって全く型の意味が異なってしまう。

これについて、単純な1引数関数を使って考える。

```python
f1: (T -> T) -> Int | T # 任意の関数を受け取り、Intを返す関数
f2: (|T: Type| T -> T) -> Int # 多相関数を受け取り、Intを返す関数
f3: Int -> (|T: Type| T -> T) # Intを受け取り、閉じた全称型関数を返す関数
f4: |T: Type|(Int -> (T -> T)) # 上と同じ意味(こちらが推奨)
```

`f3`と`f4`が同じなのに対して、`f1`と`f2`は異なるというのは奇妙に思える。実際にそのような型の関数を構成してみる。

```python
# id: |T: Type| T -> T
id x = x
# same type as `f1`
take_univq_f_and_return_i(_: (|T: Type| T -> T), i: Int): Int = i
# same type as `f2`
take_arbit_f_and_return_i|T: Type|(_: T -> T, i: Int): Int = i
# same type as `f3`
take_i_and_return_univq_f(_: Int): (|T: Type| T -> T) = id
# same type as `f4`
take_i_and_return_arbit_f|T: Type|(_: Int): (T -> T) = id
```

適用してみると、その違いがわかってくる。

```python
_ = take_univq_f_and_return_i(x -> x, 1) # OK
_ = take_univq_f_and_return_i(x: Int -> x, 1) # NG
_ = take_univq_f_and_return_i(x: Str -> x, 1) # NG
_ = take_arbit_f_and_return_i(x -> x, 1) # OK
_ = take_arbit_f_and_return_i(x: Int -> x, 1) # OK
_ = take_arbit_f_anf_return_i(x: Str -> x, 1) # OK

f: |T| T -> T = take_i_and_return_univq_f(1)
g: |T| T -> T = take_i_and_return_arbit_f(1)
assert f == g
f2: Int -> Int = take_i_and_return_univq_f|Int|(1)
g2: Int -> Int = take_i_and_return_arbit_f|Int|(1)
assert f2 == g2
```

開いた多相関数型を特に __任意関数型(arbitrary function type)__ と呼ぶ。任意関数型には、`Int -> Int`, `Str -> Str`, `Bool -> Bool`, `|T: Type| T -> T`, ...など、無限個の可能性がある。
対して閉じた(引数と同じ型のオブジェクトを返す)多相関数型は`|T: Type| T -> T`一種類のみである。このような型を特に __多相関数型(polymorphic function type)__ と呼ぶ。
換言すると、`f1`には`x: Int -> x+1`や`x: Bool -> not x`, `x -> x`などを渡すことができる=`f1`は多相関数であるが、`f2`に渡せるのは`x -> x`などのみ=`f2`は多相関数 __ではない__ 。
しかし、`f2`のような関数の型は明らかに通常の型と異なっており、これらをうまく扱える新しい概念が必要となる。それが型の「ランク」である。

ランクの定義だが、まず量化されていない型、すなわち`Int`, `Str`, `Bool`, `T`, `Int -> Int`, `Option Int`などは「ランク0」とされる。

```python
# KはOptionなどの多項カインド
R0 = (Int or Str or Bool or ...) or (R0 -> R0) or K(R0)
```

次に`|T| T -> T`など一階の全称量化が行われている型、またはそれらを戻り値型に含む型を「ランク1」とする。
さらに二階の全称量化が行われている型(`(|T| T -> T) -> Int`などランク1型を引数に持つ型)、またはそれらを戻り値型に含む型を「ランク2」とする。
以上を繰り返して「ランクN」型が定義される。また、ランクN型はN以下のランクの型をすべて含む。ゆえに、複数のランクが混在する型のランクは、その中で最も高いランクと同じになる。

```python
R1 = (|...| R0) or (R0 -> R1) or K(R1) or R0
R2 = (|...| R1) or (R1 -> R2) or K(R2) or R1
...
Rn = (|...| Rn-1) or (Rn-1 -> Rn) or K(Rn) or Rn-1
```

いくつか例をみてみよう。

```python
    (|T: Type| T -> T) -> (|U: Type| U -> U)
=>  R1 -> R1
=>  R1 -> R2
=>  R2

Option(|T: Type| T -> T)
=>  Option(R1)
=>  K(R1)
=>  R1
```

定義より、`tuple_map`はランク2型である。

```python
tuple_map:
    ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
=>  (R1, R0) -> R0
=>  R1 -> R2
=>  R2
```

Ergでは、ランク2までの型を扱うことができる(ランクN型はN以下のランクの型をすべて含むため、正確にいうとErgの型はすべてランク2型である)。それ以上の型の関数を構成しようとするとエラーになる。
例えば、多相関数を多相関数のまま扱う関数はすべて他の引数の型指定が必要である。また、このような関数は構成できない。

```python
# this is a rank-3 type function
# |X, Y: Type|((|T: Type| T -> T), (X, Y)) -> (X, Y)
generic_tuple_map|X, Y: Type| f: (|T: Type| T -> T), tup: (X, Y) = (f(tup.0), f(tup.1))
```

ランク3以上の型は理論的に型推論が決定不能となる事実が知られており、型指定は省略可能というErgの性質を崩すものであるため排除されている。とはいえ、実用的なニーズはランク2型でほとんどカバーできる。
