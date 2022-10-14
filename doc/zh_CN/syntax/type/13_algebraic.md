# 代数类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/13_algebraic.md%26commit_hash%3Dc120700585fdb1d655255c8e2817bb13cc8d369e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/13_algebraic.md&commit_hash=c120700585fdb1d655255c8e2817bb13cc8d369e)

代数类型是通过将类型视为代数来操作类型而生成的类型
它们处理的操作包括Union、Intersection、Diff、Complement等
普通类只能进行Union，其他操作会导致类型错误

## 联合(Union)

联合类型可以为类型提供多种可能性。 顾名思义，它们是由"或"运算符生成的
一个典型的 Union 是 `Option` 类型。 `Option` 类型是 `T 或 NoneType` 补丁类型，主要表示可能失败的值

```python
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# 隐式变为 `T != NoneType`
Option T = T or NoneType
```

## 路口

交集类型是通过将类型与 `and` 操作组合得到的

```python
Num = Add and Sub and Mul and Eq
```

如上所述，普通类不能与"and"操作结合使用。 这是因为实例只属于一个类

## 差异

Diff 类型是通过 `not` 操作获得的
最好使用 `and not` 作为更接近英文文本的符号，但建议只使用 `not`，因为它更适合与 `and` 和 `or` 一起使用

```python
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

## 补充

补码类型是通过 `not` 操作得到的，这是一个一元操作。 `not T` 类型是 `{=} not T` 的简写
类型为"非 T"的交集等价于 Diff，类型为"非 T"的 Diff 等价于交集
但是，不推荐这种写法

```python
# 非零数类型的最简单定义
NonZero = Not {0}
# 不推荐使用的样式
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## 真代数类型

有两种代数类型: 可以简化的表观代数类型和不能进一步简化的真实代数类型
"表观代数类型"包括 Enum、Interval 和 Record 类型的 `or` 和 `and`
这些不是真正的代数类型，因为它们被简化了，并且将它们用作类型说明符将导致警告； 要消除警告，您必须简化它们或定义它们的类型

```python
assert {1, 2, 3} or {2, 3} == {1, 2, 3}
assert {1, 2, 3} and {2, 3} == {2, 3}
assert -2..-1 or 1..2 == {-2, -1, 1, 2}

i: {1, 2} or {3, 4} = 1 # 类型警告: {1, 2} 或 {3, 4} 可以简化为 {1, 2, 3, 4}
p: {x = Int, ...} and {y = Int; ...} = {x = 1; y = 2; z = 3}
# 类型警告: {x = Int, ...} 和 {y = Int; ...} 可以简化为 {x = Int; y = 整数； ...}

Point1D = {x = Int; ...}
Point2D = Point1D and {y = Int; ...} # == {x = Int; y = Int; ...}
q: Point2D = {x = 1; y = 2; z = 3}
```

真正的代数类型包括类型"或"和"与"。 类之间的"或"等类属于"或"类型

```python
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff, Complement 类型不是真正的代数类型，因为它们总是可以被简化
