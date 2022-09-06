# 篩型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/refinement_subtyping.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/refinement_subtyping.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

篩型とは、以下のような型である。

```python
{I: Int | I >= 0}
{S: StrWithLen N | N >= 1}
{T: (Ratio, Ratio) | T.0 >= 0; T.1 >= 0}
```

ErgではEnum, Interval型を篩型に変換してしまうことで、型判定を可能にしている。

## 篩型への変換

[篩型]の項では、区間型および列挙型は、篩型の糖衣構文であると述べた。それぞれ、以下のように変換される。

* {0} -> {I: Int | I == 0}
* {0, 1} -> {I: Int | I == 0 or I == 1}
* 1.._ -> {I: Int | I >= 1}
* 1<.._ -> {I: Int | I > 1} -> {I: Int | I >= 2}
* {0} or 1.._ -> {I: Int | I == 0 or I >= 1}
* {0} or {-3, -2} or 1.._ -> {I: Int | I == 0 or (I == -2 or I == -3) or I >= 1}
* {0} and {-3, 0} -> {I: Int | I == 0 and (I == -3 or I == 0)}
* {0} not {-3, 0} or 1.._ -> {I: Int | I == 0 and not (I == -3 or I == 0) or I >= 1}

## 篩型の型判定

篩型Aが別の篩型Bのサブタイプであるか判定するアルゴリズムを説明する。形式的には、(すべての)サブタイプ判定は以下のように定義される。

```console
A <: B <=> ∀a∈A; a ∈ B
```

具体的には以下の推論規則を適用する。ブール式は簡約済みとする。

* 区間化規則(型定義から自動で行われる)
  * `Nat` => `{I: Int | I >= 0}`
* 切上規則
  * `{I: Int | I < n}` => `{I: Int | I <= n-1}`
  * `{I: Int | I > n}` => `{I: Int | I >= n+1}`
  * `{R: Ratio | R < n}` => `{R: Ratio | R <= n-ε}`
  * `{R: Ratio | R > n}` => `{R: Ratio | R >= n+ε}`
* 反転規則
  * `{A not B}` => `{A and (not B)}`
* ド・モルガンの法則
  * `{not (A or B)}` => `{not A and not B}`
  * `{not (A and B)}` => `{not A or not B}`
* 分配規則
  * `{A and (B or C)} <: D` => `{(A and B) or (A and C)} <: D` => `({A and B} <: D) and ({A and C} <: D)`
  * `{(A or B) and C} <: D` => `{(C and A) or (C and B)} <: D` => `({C and A} <: D) and ({C and B} <: D)`
  * `D <: {A or (B and C)}` => `D <: {(A or B) and (A or C)}` => `(D <: {A or B}) and (D <: {A or C})`
  * `D <: {(A and B) or C}` => `D <: {(C or A) and (C or B)}` => `(D <: {C or A}) and (D <: {C or B})`
  * `{A or B} <: C` => `({A} <: C) and ({B} <: C)`
  * `A <: {B and C}` => `(A <: {B}) and (A <: {C})`
* 終端規則
  * {I: T | ...} <: T = True
  * {} <: _ = True
  * _ <: {...} = True
  * {...} <: _ = False
  * _ <: {} == False
  * {I >= a and I <= b} (a < b) <: {I >= c} = (a >= c)
  * {I >= a and I <= b} (a < b) <: {I <= d} = (b <= d)
  * {I >= a} <: {I >= c or I <= d} (c >= d) = (a >= c)
  * {I <= b} <: {I >= c or I <= d} (c >= d) = (b <= d)
  * {I >= a and I <= b} (a <= b) <: {I >= c or I <= d} (c > d) = ((a >= c) or (b <= d))
  * 基本式
    * {I >= l} <: {I >= r} = (l >= r)
    * {I <= l} <: {I <= r} = (l <= r)
    * {I >= l} <: {I <= r} = False
    * {I <= l} <: {I >= r} = False

ブール式の簡約規則は以下の通り。min, maxは除去できない可能性がある。また、複数並んだor, andはネストしたmin, maxに変換される。

* 順序化規則
  * `I == a` => `I >= a and I <= a`
  * `i != a` => `I >= a+1 or I <= a-1`
* 恒真規則
  * `I >= a or I <= b (a < b)` == `{...}`
* 恒偽規則
  * `I >= a and I <= b (a > b)` == `{}`
* 入替規則
  * 順序式は`I >= n`, `I <= n`の順に入れ替える。
* 延長規則
  * `I == n or I >= n+1` => `I >= n`
  * `I == n or I <= n-1` => `I <= n`
* 最大規則
  * `I <= m or I <= n` => `I <= max(m, n)`
  * `I >= m and I >= n` => `I >= max(m, n)`
* 最小規則
  * `I >= m or I >= n` => `I >= min(m, n)`
  * `I <= m and I <= n` => `I <= min(m, n)`
* 除去規則
  * 左辺にある`I == n`は、右辺に`I >= a (n >= a)`か`I <= b (n <= b)`か`I == n`があるとき除去できる。
  * 左辺の等式をすべて除去できなければFalse

e.g.

```python
1.._ <: Nat
=> {I: Int | I >= 1} <: {I: Int | I >= 0}
=> {I >= 1} <: {I >= 0}
=> (I >= 0 => I >= 1)
=> 1 >= 0
=> True
# {I >= l} <: {I >= r} == (l >= r)
# {I <= l} <: {I <= r} == (l <= r)
```

```python
{I: Int | I >= 0} <: {I: Int | I >= 1 or I <= -3}
=> {I >= 0} <: {I >= 1 or I <= -3}
=> {I >= 0} <: {I >= 1} or {I >= 0} <: {I <= -3}
=> False or False
=> False
```

```python
{I: Int | I >= 0} <: {I: Int | I >= -3 and I <= 1}
=> {I >= 0} <: {I >= -3 and I <= 1}
=> {I >= 0} <: {I >= -3} and {I >= 0} <: {I <= 1}
=> True and False
=> False
```

```python
{I: Int | I >= 2 or I == -2 or I <= -4} <: {I: Int | I >= 1 or I <= -1}
=> {I >= 2 or I <= -4 or I == -2} <: {I >= 1 or I <= -1}
=>  {I >= 2 or I <= -4} <: {I >= 1 or I <= -1}
    and {I == -2} <: {I >= 1 or I <= -1}
=>      {I >= 2} <: {I >= 1 or I <= -1}
        and {I <= -4} <: {I >= 1 or I <= -1}
    and
        {I == -2} <: {I >= 1}
        or {I == -2} <: {I <= -1}
=>      {I >= 2} <: {I >= 1}
        or {I >= 2} <: {I <= -1}
    and
        {I <= -4} <: {I >= 1}
        or {I <= -4} <: {I <= -1}
    and
        False or True
=>      True or False
    and
        False or True
    and
        True
=> True and True
=> True
```
