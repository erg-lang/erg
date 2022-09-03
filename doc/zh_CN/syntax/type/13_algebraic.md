# Algebraic type（代数类型）

代数运算类型是指将类型视为代数并进行运算而产生的类型。代数类型处理的操作包括 Union、Intersection、Diff 和 Complement。常规类只能是 Union，其他操作都是类型错误。

## Union

在 Union 型中，关于型可以给出多个可能性。顾名思义，它由运算符生成。典型的 Union 是<gtr=“8”/>类型。<gtr=“9”/>类型是<gtr=“10”/>的 patch type，主要表示可能失败的值。


```erg
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# 暗黙に`T != NoneType`となる
Option T = T or NoneType
```

## Intersection

Intersection 类型是通过操作组合类型而得到的。


```erg
Num = Add and Sub and Mul and Eq
```

如上所述，无法通过操作将常规类组合在一起。实例属于唯一的类。

## Diff

Diff 类型是通过运算得到的。作为接近英文的表记，<gtr=“14”/>比较好，但由于<gtr=“15”/>，<gtr=“16”/>并列比较好，所以建议只使用<gtr=“17”/>。


```erg
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

## Complement

Complement 是通过运算得到的，但它是一元运算。<gtr=“19”/>类型是<gtr=“20”/>的缩写符号。<gtr=“21”/>类型的 Intersection 等效于 Diff，<gtr=“22”/>类型的 Diff 等效于 Intersection。但不建议这样的写法。


```erg
# the simplest definition of the non-zero number type
NonZero = Not {0}
# deprecated styles
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## 真实代数类型

代数运算类型包括可简化的表观代数运算类型和不能进一步简化的“真代数运算类型”。其他“表观代数类型”包括 Enum 和 Interval 类型，以及和<gtr=“24”/>记录类型。因为它们可以简化，所以它们不是真正的代数运算类型，如果用来指定类型，就会出现 Warning。要消除警告，必须简化或定义类型。


```erg
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

真正的代数类型包括类型和<gtr=“26”/>类型。类之间的<gtr=“27”/>等类型为<gtr=“28”/>类型。


```erg
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff 和 Complement 类型不是真正的代数运算类型，因为它们总是可以简化。
