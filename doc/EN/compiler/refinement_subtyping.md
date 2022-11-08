# Refinement type

The refinement type is the following type.

```python
{I: Int | I >= 0}
{S: StrWithLen N | N >= 1}
{T: (Ratio, Ratio) | T.0 >= 0; T.1 >= 0}
```

Erg enables type determination by converting Enum and Interval types into refinement types.

## Convert to refinement type

In the section [Refinement types], we said that interval types and enum types are syntactic sugar for refinement types. Each is converted as follows.

* {0} -> {I: Int | I == 0}
* {0, 1} -> {I: Int | I == 0 or I == 1}
* 1.._ -> {I: Int | I >= 1}
* 1<.._ -> {I: Int | I > 1} -> {I: Int | I >= 2}
* {0} or 1.._ -> {I: Int | I == 0 or I >= 1}
* {0} or {-3, -2} or 1.._ -> {I: Int | I == 0 or (I == -2 or I == -3) or I >= 1}
* {0} and {-3, 0} -> {I: Int | I == 0 and (I == -3 or I == 0)}
* {0} not {-3, 0} or 1.._ -> {I: Int | I == 0 and not (I == -3 or I == 0) or I >= 1}

## Refinement type detection

An algorithm for determining whether a refinement type A is a subtype of another refinement type B is described. Formally, (all) subtyping is defined as follows:

```console
A <: B <=> ∀a∈A; a ∈ B
```

Specifically, the following inference rules are applied. Boolean expressions are assumed to be simplified.

* intervalization rules (done automatically from type definition)
  * `Nat` => `{I: Int | I >= 0}`
* Round-up rule
  * `{I: Int | I < n}` => `{I: Int | I <= n-1}`
  * `{I: Int | I > n}` => `{I: Int | I >= n+1}`
  * `{R: Ratio | R < n}` => `{R: Ratio | R <= n-ε}`
  * `{R: Ratio | R > n}` => `{R: Ratio | R >= n+ε}`
* reversal rule
  * `{A not B}` => `{A and (not B)}`
* De Morgan's Law
  * `{not (A or B)}` => `{not A and not B}`
  * `{not (A and B)}` => `{not A or not B}`
* Distribution rule
  * `{A and (B or C)} <: D` => `{(A and B) or (A and C)} <: D` => `({A and B} <: D) and ( {A and C} <: D)`
  * `{(A or B) and C} <: D` => `{(C and A) or (C and B)} <: D` => `({C and A} <: D) and ( {C and B} <: D)`
  * `D <: {A or (B and C)}` => `D <: {(A or B) and (A or C)}` => `(D <: {A or B}) and ( D <: {A or C})`
  * `D <: {(A and B) or C}` => `D <: {(C or A) and (C or B)}` => `(D <: {C or A}) and ( D <: {C or B})`
  * `{A or B} <: C` => `({A} <: C) and ({B} <: C)`
  * `A <: {B and C}` => `(A <: {B}) and (A <: {C})`
* termination rule
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
  * basic formula
    * {I >= l} <: {I >= r} = (l >= r)
    * {I <= l} <: {I <= r} = (l <= r)
    * {I >= l} <: {I <= r} = False
    * {I <= l} <: {I >= r} = False

The simplification rules for Boolean expressions are as follows. min, max may not be removed. Also, multiple or, and are converted to nested min, max.

* ordering rules
  * `I == a` => `I >= a and I <= a`
  * `i != a` => `I >= a+1 or I <= a-1`
* Consistency rule
  * `I >= a or I <= b (a < b)` == `{...}`
* Constancy rule
  * `I >= a and I <= b (a > b)` == `{}`
* replacement rule
  * Replace order expressions in the order `I >= n` and `I <= n`.
* Extension rule
  * `I == n or I >= n+1` => `I >= n`
  * `I == n or I <= n-1` => `I <= n`
* maximum rule
  * `I <= m or I <= n` => `I <= max(m, n)`
  * `I >= m and I >= n` => `I >= max(m, n)`
* minimum rule
  * `I >= m or I >= n` => `I >= min(m, n)`
  * `I <= m and I <= n` => `I <= min(m, n)`
* elimination rule
  * `I == n` on the left side is removed when `I >= a (n >= a)` or `I <= b (n <= b)` or `I == n` on the right side can.
  * False if all left-hand equations cannot be eliminated

e.g.

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
