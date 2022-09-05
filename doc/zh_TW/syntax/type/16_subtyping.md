# 子类型

在 Erg 中，可以使用比较运算符 `<`、`>` 确定类包含。

```python
Nat < Int
Int < Object
1... _ < Nat
{1, 2} > {1}
{=} > {x = Int}
{I: Int | I >= 1} < {I: Int | I >= 0}
```

请注意，这与 `<:` 运算符的含义不同。 它声明左侧的类是右侧类型的子类型，并且仅在编译时才有意义。

```python
C <: T # T: 结构类型
f|D <: E| ...

assert F < G
```

您还可以为多态子类型规范指定 `Self <: Add`，例如 `Self(R, O) <: Add(R, O)`。

## 结构类型和类类型关系

结构类型是结构类型的类型，如果它们具有相同的结构，则被认为是相同的对象。

```python
T = Structural {i = Int}
U = Structural {i = Int}

assert T == U
t: T = {i = 1}
assert t in T
assert t in U
```

相反，类是符号类型的类型，不能在结构上与类型和实例进行比较

```python
C = Class {i = Int}
D = Class {i = Int}

assert C == D # 类型错误：无法比较类
c = C.new {i = 1}
assert c in C
assert not c in D
```

## 子程序的子类型化

子例程的参数和返回值只采用一个类。
换句话说，您不能直接将结构类型或特征指定为函数的类型。
必须使用部分类型规范将其指定为“作为该类型子类型的单个类”。

```python
# OK
f1 x, y: Int = x + y
# NG
f2 x, y: Add = x + y
# OK
# A 是一些具体的类
f3<A <: Add> x, y: A = x + y
```

子程序中的类型推断也遵循此规则。 当子例程中的变量具有未指定的类型时，编译器首先检查它是否是其中一个类的实例，如果不是，则在特征范围内查找匹配项。 如果仍然找不到，则会发生编译错误。 此错误可以通过使用结构类型来解决，但由于推断匿名类型可能会给程序员带来意想不到的后果，因此它被设计为由程序员使用 `Structural` 显式指定。

## 类向上转换

```python
i: Int
i as (Int or Str)
i as (1..10)
i as {I: Int | I >= 0}
```
