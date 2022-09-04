# 代数类型

代数类型是通过将类型视为代数来操作类型而生成的类型。
它们处理的操作包括Union、Intersection、Diff、Complement等。
普通类只能进行Union，其他操作会导致类型错误。

## 联合(Union)

联合类型可以为类型提供多种可能性。 顾名思义，它们是由“或”运算符生成的。
一个典型的 Union 是 `Option` 类型。 `Option` 类型是 `T 或 NoneType` 补丁类型，主要表示可能失败的值。


```python
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# 隐式变为 `T != NoneType`
Option T = T or NoneType
```

## 路口

交集类型是通过将类型与 `and` 操作组合得到的。

```python
Num = Add and Sub and Mul and Eq
```

如上所述，普通类不能与“and”操作结合使用。 这是因为实例只属于一个类。

## 差异

Diff 类型是通过 `not` 操作获得的。
最好使用 `and not` 作为更接近英文文本的符号，但建议只使用 `not`，因为它更适合与 `and` 和 `or` 一起使用。

```python
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

＃＃ 补充

补码类型是通过 `not` 操作得到的，这是一个一元操作。 `not T` 类型是 `{=} not T` 的简写。
类型为“非 T”的交集等价于 Diff，类型为“非 T”的 Diff 等价于交集。
但是，不推荐这种写法。

```python
# the simplest definition of the non-zero number type
NonZero = Not {0}
# deprecated styles
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## True Algebraic type

There are two algebraic types: apparent algebraic types that can be simplified and true algebraic types that cannot be further simplified.
The "apparent algebraic types" include `or` and `and` of Enum, Interval, and the Record types.
These are not true algebraic types because they are simplified, and using them as type specifiers will result in a Warning; to eliminate the Warning, you must either simplify them or define their types.

```python
assert {1, 2, 3} or {2, 3} == {1, 2, 3}
assert {1, 2, 3} and {2, 3} == {2, 3}
assert -2..-1 or 1..2 == {-2, -1, 1, 2}

i: {1, 2} or {3, 4} = 1 # TypeWarning: {1, 2} or {3, 4} can be simplified to {1, 2, 3, 4}
p: {x = Int, ...} and {y = Int; ...} = {x = 1; y = 2; z = 3}
# TypeWaring: {x = Int, ...} and {y = Int; ...} can be simplified to {x = Int; y = Int; ...}

Point1D = {x = Int; ...}
Point2D = Point1D and {y = Int; ...} # == {x = Int; y = Int; ...}
q: Point2D = {x = 1; y = 2; z = 3}
```

True algebraic types include the types `Or` and `And`. Classes such as `or` between classes are of type `Or`.

```python
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff, Complement types are not true algebraic types because they can always be simplified.
