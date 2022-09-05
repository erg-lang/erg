# 篩子類型

```python
{I: Int | I >= 0}
{S: StrWithLen N | N >= 1}
{T: (Ratio, Ratio) | T.0 >= 0; T.1 >= 0}
```

Erg 通過將 Enum 和 Interval 類型轉換為篩選類型來實現類型確定。

## 轉換為篩型

在 [Sieve types] 一節中，我們說過區間類型和枚舉類型是 sieve 類型的語法糖。每個轉換如下。

* {0} -> {I: Int | I == 0}
* {0, 1} -> {I: Int | I == 0 or I == 1}
* 1.._ -> {I: Int | I >= 1}
* 1<.._ -> {I: Int | I > 1} -> {I: Int | I >= 2}
* {0} or 1.._ -> {I: Int | I == 0 or I >= 1}
* {0} or {-3, -2} or 1.._ -> {I: Int | I == 0 or (I == -2 or I == -3) or I >= 1}
* {0} and {-3, 0} -> {I: Int | I == 0 and (I == -3 or I == 0)}
* {0} not {-3, 0} or 1.._ -> {I: Int | I == 0 and not (I == -3 or I == 0) or I >= 1}

## 篩型檢測

描述了一種用于確定篩類型 A 是否是另一篩類型 B 的子類型的算法。正式地，(所有)子類型定義如下：

```console
A <: B <=> ?a∈A; a∈B
```

具體而言，應用以下推理規則。假定布爾表達式是簡化的。

* 間隔規則(從類型定義自動完成)
  * `Nat` => `{I: Int | I >= 0}`
* 圍捕規則
  * `{I: Int | I < n}` => `{I: Int | I <= n-1}`
  * `{I: Int | I > n}` => `{I: Int | I >= n+1}`
  * `{R: Ratio | R < n}` => `{R: Ratio | R <= n-ε}`
  * `{R: Ratio | R > n}` => `{R: Ratio | R >= n+ε}`
* 反轉規則
  * `{A not B}` => `{A and (not B)}`
* 德摩根規則
  * `{not (A or B)}` => `{not A and not B}`
  * `{not (A and B)}` => `{not A or not B}`
* 分配規則
  * `{A and (B or C)} <: D` => `{(A and B) or (A and C)} <: D` => `({A and B} <: D) and ( {A and C} <: D)`
  * `{(A or B) and C} <: D` => `{(C and A) or (C and B)} <: D` => `({C and A} <: D) and ( {C and B} <: D)`
  * `D <: {A or (B and C)}` => `D <: {(A or B) and (A or C)}` => `(D <: {A or B}) and ( D <: {A or C})`
  * `D <: {(A and B) or C}` => `D <: {(C or A) and (C or B)}` => `(D <: {C or A}) and ( D <: {C or B})`
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
  * {I >= a and I <= b} (a <= b) <: {I >= c or I <= d} (c > d) = ((a >= c) or (b <= d ))
  * 基本公式
    * {I >= l} <: {I >= r} = (l >= r)
    * {I <= l} <: {I <= r} = (l <= r)
    * {I >= l} <: {I <= r} = False
    * {I <= l} <: {I >= r} = False

布爾表達式的簡化規則如下。 min, max 不能被刪除。此外，多個 or, and 被轉換為嵌套的 min, max。

* 組合規則
  * `I == a` => `I >= a 和 I <= a`
  * `i != a` => `I >= a+1 或 I <= a-1`
* 一致性規則
  * `I >= a 或 I <= b (a < b)` == `{...}`
* 恒常規則
  * `I >= a 和 I <= b (a > b)` == `{}`
* 替換規則
  * 以 `I >= n` 和 `I <= n` 的順序替換順序表達式。
* 擴展規則
  * `I == n 或 I >= n+1` => `I >= n`
  * `I == n 或 I <= n-1` => `I <= n`
* 最大規則
  * `I <= m 或 I <= n` => `I <= max(m, n)`
  * `I >= m 和 I >= n` => `I >= max(m, n)`
* 最低規則
  * `I >= m 或 I >= n` => `I >= min(m, n)`
  * `I <= m 和 I <= n` => `I <= min(m, n)`
* 淘汰規則
  * 當 `I >= a (n >= a)` 或 `I <= b (n <= b)` 或 `I == n` 在右側時，左側的 `I == n` 被刪除能夠。
  * 如果無法消除所有左手方程，則為 False

例如

```python
1.._<: Nat
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
=> {I >= 2 or I <= -4} <: {I >= 1 or I <= -1}
    and {I == -2} <: {I >= 1 or I <= -1}
=> {I >= 2} <: {I >= 1 or I <= -1}
        and {I <= -4} <: {I >= 1 or I <= -1}
    and
        {I == -2} <: {I >= 1}
        or {I == -2} <: {I <= -1}
=> {I >= 2} <: {I >= 1}
        or {I >= 2} <: {I <= -1}
    and
        {I <= -4} <: {I >= 1}
        or {I <= -4} <: {I <= -1}
    and
        False or True
=> True or False
    and
        False or True
    and
        True
=> True and True
=> True
```