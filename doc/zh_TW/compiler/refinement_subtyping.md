# 篩子型

篩型是指以下類型。


```erg
{I: Int | I >= 0}
{S: StrWithLen N | N >= 1}
{T: (Ratio, Ratio) | T.0 >= 0; T.1 >= 0}
```

在 Erg 中，通過將 Enum，Interval 型轉換為篩子型，可以進行型的判定。

## 轉換為篩子類型

在 [篩型] 一項中，區間型和列舉型是篩型的糖衣句法。分別進行如下變換。

* {0} -> {I: Int | I == 0}
* {0, 1} -> {I: Int | I == 0 or I == 1}
* 1.._ -> {I: Int | I >= 1}
* 1<.._ -> {I: Int | I > 1} -> {I: Int | I >= 2}
* {0} or 1.._ -> {I: Int | I == 0 or I >= 1}
* {0} or {-3, -2} or 1.._ -> {I: Int | I == 0 or (I == -2 or I == -3) or I >= 1}
* {0} and {-3, 0} -> {I: Int | I == 0 and (I == -3 or I == 0)}
* {0} not {-3, 0} or 1.._ -> {I: Int | I == 0 and not (I == -3 or I == 0) or I >= 1}

## 篩子型的類型判定

說明判斷篩子 A 是否是另一篩子 B 的亞型的算法。在形式上，（所有）子類型確定定義如下。


```console
A <: B <=> ∀a∈A; a ∈ B
```

具體應用以下推論規則。布爾式簡化完畢。

* 區間化規則（根據類型定義自動執行）
  * `Nat` => `{I: Int | I >= 0}`
* 切上規則
  * `{I: Int | I < n}` => `{I: Int | I <= n-1}`
  * `{I: Int | I > n}` => `{I: Int | I >= n+1}`
  * `{R: Ratio | R < n}` => `{R: Ratio | R <= n-ε}`
  * `{R: Ratio | R > n}` => `{R: Ratio | R >= n+ ε}`
* 反轉規則
  * `{A not B}` => `{A and (not B)}`
* 德-摩根定律
  * `{not (A or B)}` => `{not A and not B}`
  * `{not (A and B)}` => `{not A or not B}`
* 分配規則
  * `{A and (B or C)} <: D` => `{(A and B) or (A and C)} <: D` => `({A and B} <: D) and ({A and C} <: D)`
  * `{(A or B) and C} <: D` => `{(C and A) or (C and B)} <: D` => `({C and A} <: D) and ({C and B} <: D)`
  * `D <: {A or (B and C)}` => `D <: {(A or B) and (A or C)}` => `(D <: {A or B}) and (D <: {A or C})`
  * `D <: {(A and B) or C}` => `D <: {(C or A) and (C or B)}` => `(D <: {C or A}) and (D <: {C or B})`
  * `{A or B} <: C` => `({A} <: C) and ({B} <: C)`
  * `A <: {B and C}` => `(A <: {B}) and (A <: {C})`
* 終止規則
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
  * 基本公式
    * {I >= l} <: {I >= r} = (l >= r)
    * {I <= l} <: {I <= r} = (l <= r)
    * {I >= l} <: {I <= r} = False
    * {I <= l} <: {I >= r} = False

布爾式的簡化規則如下。 min，max 可能無法移除。此外，多個排列的 or，and 被轉換為嵌套的 min，max。

* 排序規則
  * `I == a` => `I >= a and I <= a`
  * `i!= a` => `I >= a+1 or I <= a-1`
* 恆真規則
  * `I >= a or I <= b (a < b)` == `{...}`
* 恆偽規則
  * `I >= a and I <= b (a > b)` == `{}`
* 更換規則
  * 順序表達式按，<gtr=“62”/>的順序進行替換。
* 延長規則
  * `I == n or I >= n+1` => `I >= n`
  * `I == n or I <= n-1` => `I <= n`
* 最大規則
  * `I <= m or I <= n` => `I <= max(m, n)`
  * `I >= m and I >= n` => `I >= max(m, n)`
* 最小規則
  * `I >= m or I >= n` => `I >= min(m, n)`
  * `I <= m and I <= n` => `I <= min(m, n)`
* 刪除規則
  * 左邊的，在右邊有<gtr=“76”/>或<gtr=“77”/>或<gtr=“78”/>時可以去除。
  * 如果不能移除左邊的所有等式，則返回 False

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