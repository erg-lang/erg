# 筛子型

筛型是指以下类型。


```erg
{I: Int | I >= 0}
{S: StrWithLen N | N >= 1}
{T: (Ratio, Ratio) | T.0 >= 0; T.1 >= 0}
```

在 Erg 中，通过将 Enum，Interval 型转换为筛子型，可以进行型的判定。

## 转换为筛子类型

在 [筛型] 一项中，区间型和列举型是筛型的糖衣句法。分别进行如下变换。

* {0} -> {I: Int | I == 0}
* {0, 1} -> {I: Int | I == 0 or I == 1}
* 1.._ -> {I: Int | I >= 1}
* 1<.._ -> {I: Int | I > 1} -> {I: Int | I >= 2}
* {0} or 1.._ -> {I: Int | I == 0 or I >= 1}
* {0} or {-3, -2} or 1.._ -> {I: Int | I == 0 or (I == -2 or I == -3) or I >= 1}
* {0} and {-3, 0} -> {I: Int | I == 0 and (I == -3 or I == 0)}
* {0} not {-3, 0} or 1.._ -> {I: Int | I == 0 and not (I == -3 or I == 0) or I >= 1}

## 筛子型的类型判定

说明判断筛子 A 是否是另一筛子 B 的亚型的算法。在形式上，（所有）子类型确定定义如下。


```console
A <: B <=> ∀a∈A; a ∈ B
```

具体应用以下推论规则。布尔式简化完毕。

* 区间化规则（根据类型定义自动执行）
  * `Nat` => `{I: Int | I >= 0}`
* 切上规则
  * `{I: Int | I < n}` => `{I: Int | I <= n-1}`
  * `{I: Int | I > n}` => `{I: Int | I >= n+1}`
  * `{R: Ratio | R < n}` => `{R: Ratio | R <= n-ε}`
  * `{R: Ratio | R > n}` => `{R: Ratio | R >= n+ ε}`
* 反转规则
  * `{A not B}` => `{A and (not B)}`
* 德-摩根定律
  * `{not (A or B)}` => `{not A and not B}`
  * `{not (A and B)}` => `{not A or not B}`
* 分配规则
  * `{A and (B or C)} <: D` => `{(A and B) or (A and C)} <: D` => `({A and B} <: D) and ({A and C} <: D)`
  * `{(A or B) and C} <: D` => `{(C and A) or (C and B)} <: D` => `({C and A} <: D) and ({C and B} <: D)`
  * `D <: {A or (B and C)}` => `D <: {(A or B) and (A or C)}` => `(D <: {A or B}) and (D <: {A or C})`
  * `D <: {(A and B) or C}` => `D <: {(C or A) and (C or B)}` => `(D <: {C or A}) and (D <: {C or B})`
  * `{A or B} <: C` => `({A} <: C) and ({B} <: C)`
  * `A <: {B and C}` => `(A <: {B}) and (A <: {C})`
* 终止规则
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

布尔式的简化规则如下。min，max 可能无法移除。此外，多个排列的 or，and 被转换为嵌套的 min，max。

* 排序规则
  * `I == a` => `I >= a and I <= a`
  * `i!= a` => `I >= a+1 or I <= a-1`
* 恒真规则
  * `I >= a or I <= b (a < b)` == `{...}`
* 恒伪规则
  * `I >= a and I <= b (a > b)` == `{}`
* 更换规则
  * 顺序表达式按，<gtr=“62”/>的顺序进行替换。
* 延长规则
  * `I == n or I >= n+1` => `I >= n`
  * `I == n or I <= n-1` => `I <= n`
* 最大规则
  * `I <= m or I <= n` => `I <= max(m, n)`
  * `I >= m and I >= n` => `I >= max(m, n)`
* 最小规则
  * `I >= m or I >= n` => `I >= min(m, n)`
  * `I <= m and I <= n` => `I <= min(m, n)`
* 删除规则
  * 左边的，在右边有<gtr=“76”/>或<gtr=“77”/>或<gtr=“78”/>时可以去除。
  * 如果不能移除左边的所有等式，则返回 False

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
