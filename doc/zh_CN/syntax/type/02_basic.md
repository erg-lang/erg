# 类型的基本语法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/02_basic.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/02_basic.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## 类型规范

在 Erg 中，可以在 `:` 之后指定变量的类型，如下所示。这可以与作业同时完成

```python
i: Int # 将变量 i 声明为 Int 类型
i: Int = 1
j = 1 # 类型说明可以省略
```

您还可以指定普通表达式的类型

```python
i = 1: Int
f([1, "a"]: [Int or Str])
```

对于简单的变量赋值，大多数类型说明可以省略
在定义子例程和类型时，类型规范更有用

```python
# 参数的类型规范
f x, y: Array Int = ...
T X, Y: Array Int = ...
```

请注意，在上述情况下，`x, y` 都是 `Array Int`

```python
# 大写变量的值必须是常量表达式
f X: Int = X
```

或者，如果你不需要关于类型参数的完整信息，你可以用 `_` 省略它

```python
g v: [T; _] = ...
```

但是请注意，类型规范中的 `_` 意味着 `Object`

```python
f x: _, y: Int = x + y # 类型错误: Object 和 Int 之间没有实现 +
```

## 子类型规范

除了 `:`(类型声明运算符)，Erg 还允许您使用 `<:`(部分类型声明运算符)来指定类型之间的关系
`<:` 的左边只能指定一个类。使用 `Subtypeof` 或类似的运算符来比较结构类型

这也经常在定义子例程或类型时使用，而不是简单地指定变量

```python
# 参数的子类型规范
f X <: T = ...

# 所需属性的子类型规范(.Iterator 属性必须是 Iterator 类型的子类型)
Iterable T = Trait {
    .Iterator = {Iterator} # {Iterator} == {I: Type | I <: Iterator}
    .iter = Self.() -> Self.Iterator T
    ...
}
```

也可以在定义类时使用子类型规范来静态检查该类是否是指定类型的子类型

```python
# C 类是 Show 的子类型
C = Class Object, Impl := Show
C.show self = ... # 显示所需的属性
```

您也可以仅在特定情况下指定子类型

```python
K T: Eq
K Int <: Show and Eq
K T = Class Object
K(T).
    `==` self, other = ...
K(Int).
    show self = ...
```

实现结构类型时建议使用子类型规范
这是因为，由于结构子类型的性质，拼写错误或类型规范错误在实现所需属性时不会导致错误

```python
C = Class Object
C.shoe self = ... # Show 由于 Typo 没有实现(它被认为只是一种独特的方法)
```

## 属性定义

只能在模块中为特征和类定义属性

```python
C = Class()
C.pub_attr = "this is public"
C::private_attr = "this is private"

c = C.new()
assert c.pub_attr == "this is public"
```

定义批处理定义的语法称为批处理定义，其中在 `C.` 或 `C::` 之后添加换行符，并且定义在缩进下方组合在一起

```python
C = Class()
C.pub1 = ...
C.pub2 = ...
C::priv1 = ...
C::priv2 = ...
# 相当于
C = Class()
C.
    pub1 = ...
    C. pub2 = ...
C::
    priv1 = ...
    priv2 = ...
```

## 别名

类型可以有别名。这允许缩短长类型，例如记录类型

```python
Id = Int
Point3D = {x = Int; y = Int; z = Int}
IorS = Int or Str
Vector = Array Int
```

此外，当显示错误时，如果定义了复合类型(在上面的示例中，右侧类型不是第一个类型)，编译器将为它们使用别名

但是，每个模块只允许一个相同类型的别名，多个别名将导致警告
这意味着应将具有不同用途的类型定义为单独的类型
目的还在于防止在已经具有别名的类型之上添加别名

```python
Id = Int
UserId = Int # 类型警告: 重复别名: Id 和 UserId

Ids = Array Id
Ints = Array Int # 类型警告: 重复别名: Isd 和 Ints

IorS = Int or Str
IorSorB = IorS or Bool
IorSorB_ = Int or Str or Bool # 类型警告: 重复别名: IorSorB 和 IorSorB_

Point2D = {x = Int; y = Int}
Point3D = {.... Point2D; z = Int}
Point = {x = Int; y = Int; z = Int} # 类型警告: 重复别名: Point3D 和 Point
```

<p align='center'>
    <a href='./01_type_system.md'>上一页</a> | <a href='./03_trait.md'>下一页</a>
</p>
