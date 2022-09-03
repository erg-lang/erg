# 类型的基本语法

## 类型指定

Erg 在之后指定变量类型，如下所示。也可以在赋值的同时进行。


```erg
i: Int # 声明从现在开始使用的变量 i 为 Int 类型
i: Int = 1
j = 1 # type specification can be omitted
```

也可以为常规表达式指定类型。


```erg
i = 1: Int
f([1, "a"]: [Int or Str])
```

对于简单变量赋值，大多数类型都是可选的。类型在定义子例程和类型时比简单变量更有用。


```erg
# 参数类型说明
f x, y: Array Int = ...
T X, Y: Array Int = ...
```

注意，在上述情况下，都是<gtr=“17”/>。


```erg
# 大写变量值必须是常量表达式
f X: Int = X
```

或者，如果你不完全需要类型参数信息，则可以使用将其省略。


```erg
g v: [T; _] = ...
```

但是，请注意，如果在指定类型的位置指定，则意味着<gtr=“20”/>。


```erg
f x: _, y: Int = x + y # TypeError: + is not implemented between Object and Int
```

## 子类型指定

除了使用（类型声明运算符）指定类型与表达式之间的关系外，Erg 还使用<gtr=“22”/>（子类型声明运算符）指定类型之间的关系。<gtr=“23”/>的左边只能是类。使用<gtr=“24”/>等比较结构类型。

它通常用于子程序或类型定义，而不是简单的变量。


```erg
# 部分输入参数
f X <: T = ...

# 请求属性子类型（要求 .Iterator 属性是 Iterator 类型的子类型）
Iterable T = Trait {
    .Iterator = {Iterator} # == {I | I <: Iterator}
    .iter = Self.() -> Self.Iterator T
    ...
}
```

还可以在定义类时指定子类型，以静态方式检查类是否为指定类型的子类型。


```erg
# C 类是 Show 的子类型
C = Class Object, Impl=Show
C.show self = ... # Show请求属性
```

也可以仅在特定情况下指定子类型。


```erg
K T: Eq
K Int <: Show and Eq
K T = Class Object
K(T).
    `==` self, other = ...
K(Int).
    show self = ...
```

建议在实现结构类型时使用子类型。由于结构部分类型的特性，在实现请求属性时，即使存在错误的拼贴或类型指定，也不会出现错误。


```erg
C = Class Object
C.shoe self = ... # Show 由于 Typo 没有实现（它只是被认为是一种独特的方法）
```

## 属性定义

只能在模块中为托盘和类定义属性。


```erg
C = Class()
C.pub_attr = "this is public"
C::private_attr = "this is private"

c = C.new()
assert c.pub_attr == "this is public"
```

在或<gtr=“26”/>后换行并缩进的语法称为批量定义（batch definition）。


```erg
C = Class()
C.pub1 = ...
C.pub2 = ...
C::priv1 = ...
C::priv2 = ...
# is equivalent to
C = Class()
C.
    pub1 = ...
    pub2 = ...
C::
    priv1 = ...
    priv2 = ...
```

## 锯齿

可以为类型指定别名（别名）。这使你可以将长类型（如记录类型）表示为短类型。


```erg
Id = Int
Point3D = {x = Int; y = Int; z = Int}
IorS = Int or Str
Vector = Array Int
```

此外，在错误显示过程中，编译器应尽可能使用复杂类型（在上面的示例中，不是第一种类型的右边类型）的别名。

但是，每个模块最多只能有一个别名，如果有多个别名，则会出现 warning。这意味着具有不同目的的类型应重新定义为不同的类型。它还可以防止将别名附加到已有别名的类型。


```erg
Id = Int
UserId = Int # TypeWarning: duplicate aliases: Id and UserId

Ids = Array Id
Ints = Array Int # TypeWarning: duplicate aliases: Isd and Ints

IorS = Int or Str
IorSorB = IorS or Bool
IorSorB_ = Int or Str or Bool # TypeWarning: duplicate aliases: IorSorB and IorSorB_

Point2D = {x = Int; y = Int}
Point3D = {...Point2D; z = Int}
Point = {x = Int; y = Int; z = Int} # TypeWarning: duplicate aliases: Point3D and Point
```

<p align='center'>
    <a href='./01_type_system.md'>Previous</a> | <a href='./03_trait.md'>Next</a>
</p>
