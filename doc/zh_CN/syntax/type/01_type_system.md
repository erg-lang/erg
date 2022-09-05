# Erg 的类型系统

下面简单介绍一下 Erg 的类型系统。 详细信息在其他部分进行说明。

## 如何定义

Erg 的独特功能之一是(普通)变量、函数(子例程)和类型(Kind)定义之间的语法没有太大区别。 所有都是根据普通变量和函数定义的语法定义的。

```python
f i: Int = i + 1
f # <函数 f>
f(1) # 2
f.method self = ... # 语法错误：无法为子例程定义方法

T I: Int = {...}
T # <kind 'T'>
T(1) # 类型 T(1)
T.method self = ...
D = Class {private = Int; .public = Int}
D # <类 'D'>
o1 = {private = 1; .public = 2} # o1 是一个不属于任何类的对象
o2 = D.new {private = 1; .public = 2} # o2 是 D 的一个实例
o2 = D.new {.public = 2} # 初始化错误：类 'D' 需要属性 'private'(: Int) 但未定义
```

## Classification

Erg 中的所有对象都是强类型的。
顶层类型是`{=}`，实现了`__repr__`、`__hash__`、`clone`等(不是必须的方法，这些属性不能被覆盖)。
Erg 的类型系统包含结构子类型 (SST)。 该系统类型化的类型称为结构类型。
结构类型主要分为三种：Attributive(属性类型)、Refinement(细化类型)和Algebraic(代数类型)。

|           | Record      | Enum       | Interval       | Union       | Intersection | Diff         |
| --------- | ----------- | ---------- | -------------- | ----------- | ------------ | ------------ |
| kind      | Attributive | Refinement | Refinement     | Algebraic   | Algebraic    | Algebraic    |
| generator | record      | set        | range operator | or operator | and operator | not operator |

也可以使用名义子类型(NST)，将 SST 类型转换为 NST 类型称为类型的名义化。 结果类型称为名义类型。
在 Erg 中，名义类型是类和特征。 当我们简单地说类/特征时，我们通常指的是记录类/特征。

|     | Type           | Abstraction      | Subtyping procedure |
| --- | -------------- | ---------------- | ------------------- |
| NST | NominalType    | Trait            | Inheritance         |
| SST | StructuralType | Structural Trait | (Implicit)          |

整个名义类型的类型(`NominalType`)和整个结构类型的类型(`StructuralType`)是整个类型(`Type`)的类型的子类型。

Erg 可以将参数(类型参数)传递给类型定义。带有类型参数的 `Option`、`Array` 等称为多项式类型。这些本身不是类型，但它们通过应用参数成为类型。诸如 `Int`、`Str` 等没有参数的类型称为简单类型(标量类型)。

一个类型可以看成一个集合，并且存在包含关系。例如，“Num”包含“Add”、“Sub”等，“Int”包含“Nat”。
所有类的上类是`Object == Class {:}`，所有类型的下类是`Never == Class {}`。这在下面描述。

## 类型

像 `Array T` 这样的类型可以看作是 `Type -> Type` 类型的函数，它以 `T` 类型为参数并返回 `Array T` 类型(在类型论中也称为 Kind)。像 `Array T` 这样的类型专门称为多态类型，而 `Array` 本身称为一元 Kind。

已知参数和返回类型的函数的类型表示为`(T, U) -> V`。如果要指定同一类型的整个双参数函数，可以使用 `|T| (T, T) -> T`，如果要指定整个 N 参数函数，可以使用 `Func N`。但是，`Func N` 类型没有关于参数数量或其类型的信息，因此所有返回值在调用时都是`Obj` 类型。

`Proc` 类型表示为 `() => Int` 等等。此外，`Proc` 类型实例的名称必须以 `!` 结尾。

`Method` 类型是一个函数/过程，其第一个参数是它所属的对象 `self`(通过引用)。对于依赖类型，也可以在应用方法后指定自己的类型。这是 `T!(!N)` 类型和 `T!(N ~> N-1)。 () => Int` 等等。

Erg 的数组(Array)就是 Python 所说的列表。 `[诠释; 3]`是一个数组类，包含三个`Int`类型的对象。

> __Note__: `(Type; N)` 既是类型又是值，所以可以这样使用。
>
> ```python.
> Types = (Int, Str, Bool)
>
> for! Types, T =>
>     print! T
> # Int Str Bool
> a: Types = (1, "aaa", True)
> ```

```python
pop|T, N|(l: [T; N]): ([T; N-1], T) =
    [...l, last] = l
    (l, last)

lpop|T, N|(l: [T; N]): (T, [T; N-1]) =
    [first, ...l] = l
    (first, l)
```

以 `!` 结尾的类型可以重写内部结构。 例如，`[T; !N]` 类是一个动态数组。
要从“T”类型的对象创建“T!”类型的对象，请使用一元运算符“!”。

```python
i: Int! = !1
i.update! i -> i + 1
assert i == 2
arr = [1, 2, 3]
arr.push! 4 # 导入错误
mut_arr = [1, 2, 3].into [Int; !3]
mut_arr.push4
assert mut_arr == [1, 2, 3, 4].
```

## 类型定义

类型定义如下。

```python
Point2D = {.x = Int; .y = Int}
```

请注意，如果从变量中省略 `.`，它将成为类型中使用的私有变量。 但是，这也是必需的属性。
由于类型也是对象，因此类型本身也有属性。 这样的属性称为类型属性。 在类的情况下，它们也称为类属性。

## 数据类型

如前所述，Erg 中的“类型”大致表示一组对象。

下面是 `Add` 类型的定义，需要 `+`(中间运算符)。 `R, O` 是所谓的类型参数，可以是真正的类型(类)，例如 `Int` 或 `Str`。 在其他语言中，类型参数被赋予特殊的符号(泛型、模板等)，但在 Erg 中，它们可以像普通参数一样定义。
类型参数也可以用于类型对象以外的类型。 例如数组类型`[Int; 3]` 是 `Array Int, 3` 的语法糖。 如果类型实现重叠，用户必须明确选择一个。

```python
Add R = Trait {
    .AddO = Type
    . `_+_` = Self.(R) -> Self.AddO
}
```

.`_+_`是Add.`_+_`的缩写。 前缀运算符 .`+_` 是 `Num` 类型的方法。

```python
Num = Add and Sub and Mul and Eq
NumImpl = Patch Num
NumImpl.
    `+_`(self): Self = self
    ...
```

多态类型可以像函数一样对待。 通过将它们指定为 `Mul Int、Str` 等，它们可以是单态的(在许多情况下，它们是用实际参数推断出来的，而没有指定它们)。

```python
1 + 1
`_+_` 1, 1
Nat.`_+_` 1, 1
Int.`_+_` 1, 1
```

前四行返回相同的结果(准确地说，底部的返回 `Int`)，但通常使用顶部的。
`Ratio.`_+_`(1, 1)` 将返回 `2.0` 而不会出错。
这是因为 `Int <: Ratio`，所以 `1` 向下转换为 `Ratio`。
但这不是演员。

```python
i = 1
if i: # 类型错误：i：Int 不能转换为 Bool，请改用 Int.is_zero()。
    log "a"
    log "b"
```

这是因为 `Bool <: Int` (`True == 1`, `False == 0`)。转换为子类型通常需要验证。

## 类型推理系统

Erg 使用静态鸭子类型，因此几乎不需要显式指定类型。

```python
f x, y = x + y
```

在上面的代码中，带有 `+` 的类型，即 `Add` 是自动推断的； Erg 首先推断出最小的类型。如果`f 0, 1`，它将推断`f x：{0}，y：{1}`，如果`n：Nat; f n, 1`，它会推断`f x: Nat, y: {1}`。最小化之后，增加类型直到找到实现。在 `{0}, {1}` 的情况下，`Nat` 与 `Nat` 是单态的，因为 `Nat` 是具有 `+` 实现的最小类型。
如果是 `{0}, {-1}`，它与 `Int` 是单态的，因为它不匹配 `Nat`。如果子类型和超类型之间没有关系，则首先尝试具有最低浓度(实例数)(或者在多态类型的情况下参数更少)的那个。
`{0}` 和 `{1}` 是枚举类型，它们是部分类型，例如 `Int` 和 `Nat`。
例如，可以为枚举类型指定名称和请求/实现方法。在有权访问该类型的命名空间中，满足请求的对象可以使用实现方法。

```python
Binary = Patch {0, 1}
Binary.
    # self 包含一个实例。 在此示例中，为 0 或 1。
    # 如果你想重写self，你必须追加！ 必须添加到类型名称和方法名称。
    is_zero(self) = match self:
        0 -> True
        1 -> False # 你也可以使用 _ -> False
    is_one(self) = not self.is_zero()
    to_bool(self) = match self:
        0 -> False
        1 -> True
```

此后，代码“0.to_bool()”是可能的(尽管“0 as Bool == False”是内置定义的)。
这是一个实际上可以重写 `self` 的类型的示例，如代码所示。

```python
Binary! = Patch {0, 1}!
Binary!
    switch! ref! self = match! self:
        0 => self = 1
        1 => self = 0

b = !1
b.switch!()
print! b # => 0
```

## 结构类型(匿名类型)

```python
Binary = {0, 1}
```

上面代码中的 `Binary` 是一个类型，其元素是 `0` 和 `1`。 它也是 `Int` 类型的子类型，它同时具有 `0` 和 `1`。
像 `{}` 这样的对象本身就是一种类型，可以在分配或不分配给上述变量的情况下使用。
这样的类型称为结构类型。 当我们想强调它作为后者而不是类(命名类型)的用途时，它也被称为未命名类型。 `{0, 1}`这样的结构类型称为枚举类型，还有区间类型、记录类型等。

### 类型标识

无法指定以下内容。 例如，您不能指定 `Int` 和 `Int` 和 `Int` 和 `Int` 和 `Int` 和 `Int`。
例如，`Int`和`Str`都是`Add`，但是`Int`和`Str`不能相加。

```python
add l: Add, r: Add =
    l + r # 类型错误: `_+_` 没有实现: |T, U <: Add| (T, U) -> <失败>
```

此外，下面的类型 `A` 和 `B` 不被认为是同一类型。 但是，类型“O”被认为是匹配的

```python
... |R1; R2; O; A <: Add(R1, O); B <: Add(R2, O)|
```

<p align='center'>
    上一页 | <a href='./02_basic.md'>下一页</a>
</p>
