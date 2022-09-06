# 筛子类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/refinement_subtyping.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/refinement_subtyping.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

```python
{I: Int | I >= 0}
{S: StrWithLen N | N >= 1}
{T: (Ratio, Ratio) | T.0 >= 0; T.1 >= 0}
```

Erg 通过将 Enum 和 Interval 类型转换为筛选类型来实现类型确定。

## 转换为筛型

在 [Sieve types] 一节中，我们说过区间类型和枚举类型是 sieve 类型的语法糖。每个转换如下。

* {0} -> {I: Int | I == 0}
* {0, 1} -> {I: Int | I == 0 or I == 1}
* 1.._ -> {I: Int | I >= 1}
* 1<.._ -> {I: Int | I > 1} -> {I: Int | I >= 2}
* {0} or 1.._ -> {I: Int | I == 0 or I >= 1}
* {0} or {-3, -2} or 1.._ -> {I: Int | I == 0 or (I == -2 or I == -3) or I >= 1}
* {0} and {-3, 0} -> {I: Int | I == 0 and (I == -3 or I == 0)}
* {0} not {-3, 0} or 1.._ -> {I: Int | I == 0 and not (I == -3 or I == 0) or I >= 1}

## 筛型检测

描述了一种用于确定筛类型 A 是否是另一筛类型 B 的子类型的算法。正式地，(所有)子类型定义如下：

```console
A <: B <=> ∀a∈A; a∈B
```

具体而言，应用以下推理规则。假定布尔表达式是简化的。

* 间隔规则(从类型定义自动完成)
  * `Nat` => `{I: Int | I >= 0}`
* 围捕规则
  * `{I: Int | I < n}` => `{I: Int | I <= n-1}`
  * `{I: Int | I > n}` => `{I: Int | I >= n+1}`
  * `{R: Ratio | R < n}` => `{R: Ratio | R <= n-ε}`
  * `{R: Ratio | R > n}` => `{R: Ratio | R >= n+ε}`
* 反转规则
  * `{A not B}` => `{A and (not B)}`
* 德摩根规则
  * `{not (A or B)}` => `{not A and not B}`
  * `{not (A and B)}` => `{not A or not B}`
* 分配规则
  * `{A and (B or C)} <: D` => `{(A and B) or (A and C)} <: D` => `({A and B} <: D) and ( {A and C} <: D)`
  * `{(A or B) and C} <: D` => `{(C and A) or (C and B)} <: D` => `({C and A} <: D) and ( {C and B} <: D)`
  * `D <: {A or (B and C)}` => `D <: {(A or B) and (A or C)}` => `(D <: {A or B}) and ( D <: {A or C})`
  * `D <: {(A and B) or C}` => `D <: {(C or A) and (C or B)}` => `(D <: {C or A}) and ( D <: {C or B})`
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
  * {I >= a and I <= b} (a <= b) <: {I >= c or I <= d} (c > d) = ((a >= c) or (b <= d ))
  * 基本公式
    * {I >= l} <: {I >= r} = (l >= r)
    * {I <= l} <: {I <= r} = (l <= r)
    * {I >= l} <: {I <= r} = False
    * {I <= l} <: {I >= r} = False

布尔表达式的简化规则如下。 min, max 不能被删除。此外，多个 or, and 被转换为嵌套的 min, max。

* 组合规则
  * `I == a` => `I >= a 和 I <= a`
  * `i != a` => `I >= a+1 或 I <= a-1`
* 一致性规则
  * `I >= a 或 I <= b (a < b)` == `{...}`
* 恒常规则
  * `I >= a 和 I <= b (a > b)` == `{}`
* 替换规则
  * 以 `I >= n` 和 `I <= n` 的顺序替换顺序表达式。
* 扩展规则
  * `I == n 或 I >= n+1` => `I >= n`
  * `I == n 或 I <= n-1` => `I <= n`
* 最大规则
  * `I <= m 或 I <= n` => `I <= max(m, n)`
  * `I >= m 和 I >= n` => `I >= max(m, n)`
* 最低规则
  * `I >= m 或 I >= n` => `I >= min(m, n)`
  * `I <= m 和 I <= n` => `I <= min(m, n)`
* 淘汰规则
  * 当 `I >= a (n >= a)` 或 `I <= b (n <= b)` 或 `I == n` 在右侧时，左侧的 `I == n` 被删除能够。
  * 如果无法消除所有左手方程，则为 False

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