# 继承

继承允许您定义一个新类，为现有类添加功能或专业化。
继承类似于包含在特征中。 继承的类成为原始类的子类型。

```python
NewInt = Inherit Int
NewInt.
    plus1 self = self + 1

assert NewInt.new(1).plus1() == 2
assert NewInt.new(1) + NewInt.new(1) == 2
```

如果你希望新定义的类是可继承的，你必须给它一个 `Inheritable` 装饰器。

您可以指定一个可选参数 `additional` 以允许该类具有其他实例属性，但前提是该类是一个值类。 但是，如果类是值类，则不能添加实例属性。

```python
@Inheritable
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}

john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

MailAddress = Inherit Str, additional: {owner = Str} # 类型错误：实例变量不能添加到值类中
```

Erg 的特殊设计不允许继承“Never”类型。 Erg 的特殊设计不允许继承 `Never` 类型，因为 `Never` 是一个永远无法实例化的独特类。

## 枚举类的继承

[Or 类型](./13_algebraic.md) 也可以被继承。 在这种情况下，您可以通过指定可选参数 `Excluding` 来删除任何选项（可以使用 `or` 进行多项选择）。
不能添加其他选项。 添加选项的类不是原始类的子类型。

```python
Number = Class Int or Float or Complex
Number.abs(self): Float =
    match self:
        i: Int -> i.abs().into Float
        f: Float -> f.abs()
        c: Complex -> c.abs().into Float

# c: 复杂不能出现在匹配选项中
RealNumber = Inherit Number, Excluding: Complex
```

同样，也可以指定[细化类型](./12_refinement.md)。

```python
Months = Class 0..12
MonthsNot31Days = Inherit Months, Excluding: {1, 3, 5, 7, 8, 10, 12}

StrMoreThan3 = Class StrWithLen N | N >= 3
StrMoreThan4 = Inherit StrMoreThan3, Excluding: StrWithLen N | N == 3
```

## 覆盖

该类与补丁相同，可以将新方法添加到原始类型，但可以进一步“覆盖”该类。
这种覆盖称为覆盖。要覆盖，必须满足三个条件。
首先，覆盖必须有一个 `Override` 装饰器，因为默认情况下它会导致错误。
另外，覆盖不能改变方法的类型。它必须是原始类型的子类型。
如果你重写了一个被另一个方法引用的方法，你也必须重写所有被引用的方法。

为什么这个条件是必要的？这是因为重写不仅会改变一种方法的行为，而且可能会影响另一种方法的行为。

让我们从第一个条件开始。此条件是为了防止“意外覆盖”。
换句话说，必须使用 `Override` 装饰器来防止派生类中新定义的方法的名称与基类的名称冲突。

接下来，考虑第二个条件。这是为了类型一致性。由于派生类是基类的子类型，因此它的行为也必须与基类的行为兼容。

最后，考虑第三个条件。这种情况是 Erg 独有的，在其他面向对象语言中并不常见，同样是为了安全。让我们看看如果不是这种情况会出现什么问题。

```python
# 反面示例
@Inheritable
Base! = Class {x = Int!}
Base!
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!
    @Override
    g! ref! self = self.f!() # 无限递归警告：此代码陷入无限循环 
    # 覆盖错误：方法 `.g` 被 `.f` 引用但未被覆盖
```

在继承类 `Inherited!` 中，`.g!` 方法被重写以将处理转移到 `.f!`。 但是，基类中的 `.f!` 方法会将其处理转移到 `.g!`，从而导致无限循环。 `.f` 是 `Base!` 类中的一个没有问题的方法，但它被覆盖以一种意想不到的方式使用，并且被破坏了。

Erg 已将此规则构建到规范中。

```python
# OK.
@Inheritable
Base! = Class {x = Int!}
Base!
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!
    @Override
    f! ref! self =
        print! self::x
        self::x.update! x -> x + 1
    @Override
    g! ref! self = self.f!()
```

然而，这个规范并没有完全解决覆盖问题。 然而，这个规范并没有完全解决覆盖问题，因为编译器无法检测覆盖是否解决了问题。
创建派生类的程序员有责任纠正覆盖的影响。 只要有可能，尝试定义一个别名方法。

### 替换特征（或看起来像什么）

尽管无法在继承时替换特征，但有一些示例似乎可以这样做。

例如，`Int`，`Real` 的子类型（实现了 `Add()`），似乎重新实现了 `Add()`。

```python
Int = Class ... , Impl := Add() and ...
```

但实际上 `Real` 中的 `Add()` 代表 `Add(Real, Real)`，而在 `Int` 中它只是被 `Add(Int, Int)` 覆盖。
它们是两个不同的特征（`Add` 是一个 [covariate](./advanced/variance.md)，所以`Add(Real, Real) :> Add(Int, Int)`）。

## 多重继承

Erg 不允许普通类之间的交集、差异和互补。
```python
Int and Str # 类型错误：无法合并类
```

该规则防止从多个类继承，即多重继承。

```python
IntAndStr = Inherit Int and Str # 语法错误：不允许类的多重继承
```

但是，可以使用多个继承的 Python 类。

## 多层（多级）继承

Erg 继承也禁止多层继承。 也就是说，您不能定义从另一个类继承的类。
从“Object”继承的可继承类可能会异常继承。

同样在这种情况下，可以使用 Python 的多层继承类。

## 重写继承的属性

Erg 不允许重写从基类继承的属性。 这有两个含义。

第一个是对继承的源类属性的更新操作。 例如，它不能重新分配，也不能通过 `.update!` 方法更新。

覆盖与重写不同，因为它是一种用更专业的方法覆盖的操作。 覆盖也必须替换为兼容的类型。

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!
    var = !1
    inc_pub! ref! self = self.pub.update! p -> p + 1

Inherited! = Inherit Base!
Inherited!
    var.update! v -> v + 1
    # 类型错误：不能更新基类变量
    @Override
    inc_pub! ref! self = self.pub + 1
    # 覆盖错误：`.inc_pub!` 必须是 `Self! 的子类型！ () => ()`
```

第二个是对继承源的（变量）实例属性的更新操作。 这也是被禁止的。 基类的实例属性只能从基类提供的方法中更新。
无论属性的可见性如何，都无法直接更新。 但是，它们可以被读取。

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!
    inc_pub! ref! self = self.pub.update! p -> p + 1
    inc_pri! ref! self = self::pri.update! p -> p + 1

self = self.pub.update!
Inherited!
    # OK
    add2_pub! ref! self =
        self.inc_pub!()
        self.inc_pub!()
    # NG, `Child` 不能触摸 `self.pub` 和 `self::pri`。
    add2_pub! ref! self =
        self.pub.update! p -> p + 2
```

毕竟 Erg 继承只能添加新的属性和覆盖基类的方法。

## 使用继承

虽然继承在正确使用时是一项强大的功能，但它也有一个缺点，即它往往会使类依赖关系复杂化，尤其是在使用多层或多层继承时。复杂的依赖关系会降低代码的可维护性。
Erg 禁止多重和多层继承的原因是为了降低这种风险，并且引入了类补丁功能以降低依赖关系的复杂性，同时保留继承的“添加功能”方面。

那么，反过来说，应该在哪里使用继承呢？一个指标是何时需要“基类的语义子类型”。
Erg 允许类型系统自动进行部分子类型确定（例如，Nat，其中 Int 大于或等于 0）。
但是，例如，仅依靠 Erg 的类型系统很难创建“表示有效电子邮件地址的字符串类型”。您可能应该对普通字符串执行验证。然后，我们想为已通过验证的字符串对象添加某种“保证”。这相当于向下转换为继承的类。将 `Str object` 向下转换为 `ValidMailAddressStr` 与验证字符串是否采用正确的电子邮件地址格式是一一对应的。

```python
ValidMailAddressStr = Inherit Str
ValidMailAddressStr.
    init s: Str =
        validate s # 邮件地址验证
        Self.new s

s1 = "invalid mail address"
s2 = "foo@gmail.com"
_ = ValidMailAddressStr.init s1 # 恐慌：无效的邮件地址
valid = ValidMailAddressStr.init s2
valid: ValidMailAddressStr # 确保电子邮件地址格式正确
```

另一个指标是您何时想要实现名义多态性。
例如，下面定义的 `greet!` 过程将接受任何类型为 `Named` 的对象。
但显然应用 `Dog` 类型的对象是错误的。 所以我们将使用 `Person` 类作为参数类型。
这样，只有 `Person` 对象、从它们继承的类和 `Student` 对象将被接受为参数。
这是比较保守的，避免不必要地承担过多的责任。

```python
Named = {name = Str; ...}
Dog = Class {name = Str; breed = Str}
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}
structural_greet! person: Named =
    print! "Hello, my name is {person::name}."
greet! person: Person =
    print! "Hello, my name is {person::name}."

max = Dog.new {name = "Max", breed = "Labrador"}
john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

structural_greet! max # 你好，我是马克斯
structural_greet! john # 你好，我是约翰
greet! alice # 你好，我是爱丽丝
greet! max # 类型错误：
```

<p align='center'>
    <a href='./04_class.md'>上一页</a> | <a href='./06_nst_vs_sst.md'>下一页</a>
</p>
