# Erg 类类型系统

下面是 Erg 类型系统的简要说明。其他部分将介绍更多信息。

## 定义方法

Erg 的独特之处在于，（常规）变量、函数（子程序）和类型（卡印度）的定义没有太大的语法差异。所有这些都是根据常规变量和函数定义的语法定义的。


```erg
f i: Int = i + 1
f # <function f>
f(1) # 2
f.method self = ... # SyntaxError: cannot define a method to a subroutine

T I: Int = {...}
T # <kind 'T'>
T(1) # Type T(1)
T.method self = ...
D = Class {private = Int; .public = Int}
D # <class 'D'>
o1 = {private = 1; .public = 2} # o1はどのクラスにも属さないオブジェクト
o2 = D.new {private = 1; .public = 2} # o2はDのインスタンス
o2 = D.new {.public = 2} # InitializationError: class 'D' requires attribute 'private'(: Int) but not defined
```

## 分类

Erg 中的所有对象都已输入。最高类型是，它实现了<gtr=“16”/>，<gtr=“17”/>，<gtr=“18”/>等（它们不是请求方法，也不能覆盖这些属性）。Erg 类型系统采用结构子类型（Structural subtyping，SST）。系统输入的类型称为“结构类型”（Structural type）。有三种结构类型：Attributive（属性类型）/Refinement（筛子类型）/Algebraic（代数类型）。

|           | Record      | Enum       | Interval       | Union       | Intersection | Diff         |
| --------- | ----------- | ---------- | -------------- | ----------- | ------------ | ------------ |
| kind      | Attributive | Refinement | Refinement     | Algebraic   | Algebraic    | Algebraic    |
| generator | record      | set        | range operator | or operator | and operator | not operator |

也可以使用 Nominal subtyping（Nominal subtyping，NST），将 SST 类型转换为 NST 类型称为“类型记名”（Nominalization）。这种类型称为“记名类型”（Nominal type）。在 Erg 中，记名类型为类和trait。如果只是一个类/任务，则通常指的是记录类/记录任务。

|     | Type           | Abstraction      | Subtyping procedure |
| --- | -------------- | ---------------- | ------------------- |
| NST | NominalType    | Trait            | Inheritance         |
| SST | StructuralType | Structural Trait | (Implicit)          |

表示整个记名类型的类型（）和整个结构类型的类型（<gtr=“20”/>）是整个类型的类型（<gtr=“21”/>）的子类型。

Erg 可以将参数（类型参数）传递给类型定义。具有类型参数的，<gtr=“23”/>等称为多项卡印。它们本身不是类型，但通过应用参数成为类型。此外，没有参数的<gtr=“24”/>或<gtr=“25”/>类型称为简单类型（标量类型）。

类型可以被视为一个集合，也存在包含关系。例如，包含<gtr=“27”/>和<gtr=“28”/>等，<gtr=“29”/>包含<gtr=“30”/>。所有类的上级类为<gtr=“31”/>，所有类型的下级类为<gtr=“32”/>。我们将在后面讨论这一点。

## 型

像这样的类型以<gtr=“34”/>为参数，返回<gtr=“35”/>类型，即<gtr=“36”/>类型的函数（理论上也称为类型）。像<gtr=“37”/>这样的类型被特别称为多相类型，而<gtr=“38”/>本身被称为 1 项卡印度。

参数和返回类型已知的函数类型将显示为。如果要指定类型相同的 2 自变量函数整体，可以指定<gtr=“40”/>；如果要指定 N 自变量函数整体，可以指定<gtr=“41”/>。但是，由于<gtr=“42”/>类型没有关于参数数量或类型的信息，因此调用时所有返回值都是<gtr=“43”/>类型。

类型应表示为<gtr=“45”/>，依此类推。此外，<gtr=“46”/>类型实例的名称必须以<gtr=“47”/>结尾。

类型是一个函数/过程，它将其所属的对象<gtr=“49”/>指定为第一个参数（作为引用）。对于依赖关系，你还可以在应用方法后指定自己的类型。这意味着你可以指定<gtr=“50”/>类型的方法，例如<gtr=“51”/>。

Erg 数组（Array）就是 Python 的列表。是包含三个<gtr=“53”/>类型对象的数组类。

> ：<gtr=“54”/>既是类型又是值，因此可以这样使用。
>
> `` `erg
> Types = (Int, Str, Bool)
>
> for! Types, T =>
>     print! T
> # Int Str Bool
> a: Types = (1, "aaa", True)
> ```


```erg
pop|T, N|(l: [T; N]): ([T; N-1], T) =
    [...l, last] = l
    (l, last)

lpop|T, N|(l: [T; N]): (T, [T; N-1]) =
    [first, ...l] = l
    (first, l)
```

带有的类型允许对象的内部结构重写。例如，<gtr=“58”/>类是一个动态数组。要从<gtr=“59”/>类型对象生成<gtr=“60”/>类型对象，请使用一元运算符<gtr=“61”/>。


```erg
i: Int! = !1
i.update! i -> i + 1
assert i == 2
arr = [1, 2, 3]
arr.push! 4 # ImplError:
mut_arr = [1, 2, 3].into [Int; !3]
mut_arr.push! 4
assert mut_arr == [1, 2, 3, 4]
```

## 类型定义

类型定义如下。


```erg
Point2D = {.x = Int; .y = Int}
```

如果省略，例如<gtr=“62”/>，则它将成为类型中使用的私有变量。但这也是请求属性。类型本身也有属性，因为类型也是对象。这些属性称为类型属性。类也称为类属性。

## 类型类、数据类型（等效）

如前所述，Erg 中的“类型”大致是指一组对象。以下是要求（中置运算符）的<gtr=“65”/>类型的定义。<gtr=“66”/>是一个所谓的类型参数，它包含实现的类型（类），如<gtr=“67”/>和<gtr=“68”/>。在其他语言中，类型参数具有特殊的符号（通用、模板等），但在 Erg 中，类型参数的定义方式与常规参数的定义方式相同。类型参数也可以不是类型对象。例如，序列类型是的语法糖。如果类型实现被覆盖，则用户必须显式选择。


```erg
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
```

.是 Add.<gtr=“72”/>的缩写。前缀运算符.<gtr=“73”/>是类型为<gtr=“74”/>的方法。


```erg
Num = Add and Sub and Mul and Eq
NumImpl = Patch Num
NumImpl.
    `+_`(self): Self = self
    ...
```

多相类型可以像函数一样处理。单相化，例如（在许多情况下，即使未指定，也会使用实际参数进行推理）。


```erg
1 + 1
`_+_` 1, 1
Nat.`_+_` 1, 1
Int.`_+_` 1, 1
```

最上面的四行返回相同的结果（确切地说，最下面的行返回），但通常使用最上面的行。

```Ratio.`_+_`(1, 1)```とすると、エラーにはならず`2.0`が返ります。
これは、`Int <: Ratio`であるために`1`が`Ratio`にダウンキャストされるからです。
しかしこれはキャストされません。

```erg
i = 1
if i: # TypeError: i: Int cannot cast to Bool, use Int.is_zero() instead.
    log "a"
    log "b"
```

这是因为（<gtr=“78”/>，<gtr=“79”/>）。转换到子类型通常需要验证。

## 类型推理系统

Erg 采用静态烤鸭打字，几乎不需要明确指定类型。


```erg
f x, y = x + y
```

对于上面的代码，将自动推断具有的类型，即<gtr=“81”/>。Erg 首先推论最小的类型。如果<gtr=“82”/>，则推论为<gtr=“83”/>；如果<gtr=“84”/>，则推论为<gtr=“85”/>。最小化后，类型将不断增大，直到找到实现。对于<gtr=“86”/>，由于<gtr=“87”/>是具有<gtr=“88”/>实现的最小类型，因此将单相化为<gtr=“89”/>。<gtr=“90”/>与<gtr=“91”/>不匹配，因此将单相化为<gtr=“92”/>。如果不是子类型或上类型关系，则从浓度（实例数）较低（如果是多相类型，则参数更少）开始尝试。<gtr=“93”/>和<gtr=“94”/>是作为<gtr=“95”/>和<gtr=“96”/>等部分类型的枚举类型。枚举类型等可以命名为请求/实现方法。在可以访问该类型的命名空间中，满足请求的对象可以使用实现方法。


```erg
Binary = Patch {0, 1}
Binary.
    # selfにはインスタンスが格納される。この例では0か1のどちらか。
    # selfを書き換えたい場合、型名、メソッド名に!を付けなければならない。
    is_zero(self) = match self:
        0 -> True
        1 -> False # _ -> Falseとしてもよい
    is_one(self) = not self.is_zero()
    to_bool(self) = match self:
        0 -> False
        1 -> True
```

以下代码可能是（尽管<gtr=“98”/>是内置定义的）。如代码中所示，下面是一个类型的示例，该类型实际上可以重写<gtr=“99”/>。


```erg
Binary! = Patch {0, 1}!
Binary!.
    switch! ref! self = match! self:
        0 => self = 1
        1 => self = 0

b = !1
b.switch!()
print! b # => 0
```

## 结构（未命名）


```erg
Binary = {0, 1}
```

在上面的代码中，是元素的类型，其中<gtr=“101”/>和<gtr=“102”/>是元素的类型。也可以说是既有<gtr=“103”/>又有<gtr=“104”/>的<gtr=“105”/>类型的子类型。像这样的对象本身就是一个类型，可以像上面那样代入变量使用，也可以不代入变量使用。这种类型称为结构类型。与类（记名型）对比，强调作为后者使用时，也称为无名型。像这样的结构类型称为枚举类型，其他类型包括区间类型和记录类型。

### 类型同一性

不能像下面这样指定。被解释为指的是不同的东西。例如，<gtr=“109”/>和<gtr=“110”/>都是<gtr=“111”/>，但<gtr=“112”/>和<gtr=“113”/>不能相加。


```erg
add l: Add, r: Add =
    l + r # TypeError: there is no implementation of  `_+_`: |T, U <: Add| (T, U) -> <Failure>
```

此外，下面的和<gtr=“115”/>不能被视为同一类型。但是，类型<gtr=“116”/>被视为匹配。


```erg
... |R1; R2; O; A <: Add(R1, O); B <: Add(R2, O)|
```

<p align='center'>
    Previous | <a href='./02_basic.md'>Next</a>
</p>
