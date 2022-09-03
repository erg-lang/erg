# 继承（Inheritance）

通过继承，你可以定义一个新类，该新类将添加或特定于现有类。继承类似于特雷特中的包容。继承的类将成为原始类的子类型。


```erg
NewInt = Inherit Int
NewInt.
    plus1 self = self + 1

assert NewInt.new(1).plus1() == 2
assert NewInt.new(1) + NewInt.new(1) == 2
```

如果你希望新定义的类是可继承类，则必须指定装饰器。

可选参数允许你具有其他实例属性。但是，不能为值类添加实例属性。


```erg
@Inheritable
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}

john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

MailAddress = Inherit Str, additional: {owner = Str} # TypeError: instance variables cannot be added to a value class
```

Erg 中例外的是不能继承型的设计。因为<gtr=“17”/>是绝对不能生成实例的特殊类。

## 枚举类继承

也可以继承以为类的枚举类。在这种情况下，你可以通过指定选项参数<gtr=“18”/>来删除任何选项（使用<gtr=“19”/>可以选择多个选项）。仍不能添加。添加选择的类不是原始类的子类型。


```erg
Number = Class Int or Float or Complex
Number.
    abs(self): Float =
        match self:
            i: Int -> i.abs().into Float
            f: Float -> f.abs()
            c: Complex -> c.abs().into Float

# matchの選択肢でc: Complexは現れ得ない
RealNumber = Inherit Number, Excluding: Complex
```

同样，也可以指定。


```erg
Months = Class 0..12
MonthsNot31Days = Inherit Months, Excluding: {1, 3, 5, 7, 8, 10, 12}

StrMoreThan3 = Class StrWithLen N | N >= 3
StrMoreThan4 = Inherit StrMoreThan3, Excluding: StrWithLen N | N == 3
```

## 覆盖

与修补程序相同，你可以在原始类型中添加新方法，但可以进一步“覆盖”类。覆盖称为覆盖。覆盖必须满足三个条件。首先，默认情况下，覆盖是错误的，因此必须添加装饰器。此外，覆盖不能更改方法类型。必须是原始类型的子类型。如果要覆盖其他方法引用的方法，则必须覆盖所有引用的方法。

为什么要有这样的条件呢？这是因为覆盖不仅可以改变一个方法的行为，还可以影响另一个方法的行为。

首先，从第一个条件开始解说。这是为了防止“意外覆盖”。这意味着必须在装饰器中显示，以防止派生类中新定义的方法的名称碰巧与基类冲突。

接下来，我们考虑第二个条件。这是为了保持类型的完整性。派生类是基类的子类型，因此其行为也必须与基类兼容。

最后，考虑第三个条件。这个条件是 Erg 特有的，在其他面向对象的语言中并不常见，但这也是为了安全起见。看看没有这个的时候会发生什么不好的事情。


```erg
# Bad example
@Inheritable
Base! = Class {x = Int!}
Base!.
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!.
    @Override
    g! ref! self = self.f!() # InfiniteRecursionWarning: This code falls into an infinite loop
    # OverrideError: method `.g` is referenced by `.f` but not overridden
```

继承类覆盖<gtr=“25”/>方法并将处理转发到<gtr=“26”/>。但是，基类的<gtr=“27”/>方法将其处理转发到<gtr=“28”/>，从而导致无限循环。<gtr=“29”/>在<gtr=“30”/>类中是一个没有问题的方法，但由于被覆盖而被意外地使用，并被破坏。

因此，通常需要重写所有可能受覆盖影响的方法。Erg 将这一规则纳入规范。


```erg
# OK
@Inheritable
Base! = Class {x = Int!}
Base!.
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!.
    @Override
    f! ref! self =
        print! self::x
        self::x.update! x -> x + 1
    @Override
    g! ref! self = self.f!()
```

但这一规范并不能完全解决覆盖问题。编译器无法检测覆盖是否修复了问题。创建派生类的程序员有责任修改替代的影响。应尽可能定义别名方法。

### 替换特雷特（类似于）

你不能在继承过程中替换 TRAIT，但有一个示例似乎是这样做的。

例如，（实现<gtr=“32”/>）的子类型<gtr=“33”/>似乎正在重新实现<gtr=“34”/>。


```erg
Int = Class ..., Impl := Add() and ...
```

但实际上，中的<gtr=“36”/>是<gtr=“37”/>的缩写，<gtr=“38”/>只是用<gtr=“39”/>覆盖。两者是不同的特雷特（<gtr=“40”/>是<gtr=“42”/>，因此<gtr=“41”/>）。

## 禁止多重继承

Erg 不允许常规类之间的 Intersection、Diff 或 Complement。


```erg
Int and Str # TypeError: cannot unite classes
```

此规则不允许继承多个类，即多重继承。


```erg
IntAndStr = Inherit Int and Str # SyntaxError: multiple inheritance of classes is not allowed
```

但是，可以使用 Python 多重继承类。

## 禁止多层继承

Erg 继承也禁止多层继承。也就是说，你不能定义继承的类，也不能定义继承的类。但是，继承的（Inheritable）类除外。

此外，Python 多层继承类仍然可用。

## 禁止改写源属性

Erg 无法重写源属性。这有两个意思。首先，对继承的类属性执行更新操作。不仅不能重新赋值，也不能通过方法进行更新。

覆盖与重写不同，因为它是一种使用更特定的方法进行覆盖的操作。替代也必须使用兼容类型进行替换。


```erg
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!.
    var = !1
    inc_pub! ref! self = self.pub.update! p -> p + 1

Inherited! = Inherit Base!:
Inherited!.
    var.update! v -> v + 1
    # TypeError: can't update base class variables
    @Override
    inc_pub! ref! self = self.pub + 1
    # OverrideError: `.inc_pub!` must be subtype of `Self!.() => ()`
```

第二种是对从其继承的（可变）实例属性执行更新操作。这也是禁止的。只能从基类提供的方法更新基类的实例属性。无论属性的可视性如何，都不能直接更新。但是可以读取。


```erg
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!.
    inc_pub! ref! self = self.pub.update! p -> p + 1
    inc_pri! ref! self = self::pri.update! p -> p + 1

Inherited! = Inherit Base!:
Inherited!.
    # OK
    add2_pub! ref! self =
        self.inc_pub!()
        self.inc_pub!()
    # NG, `Child` cannot touch `self.pub` and `self::pri`
    add2_pub! ref! self =
        self.pub.update! p -> p + 2
```

最后，Erg 只能继承添加新属性和覆盖基类方法。

## 继承用法

如果正确使用，继承是一个强大的功能，但另一方面，它也有一个缺点，即类之间的依赖关系容易变得复杂，特别是在使用多重继承和多层继承时，这种趋势更为明显。依赖项的复杂性可能会降低代码的可维护性。Erg 禁止多重继承和多层继承是为了降低这种风险，而引入类修补功能是为了在继承“添加功能”的同时减少依赖关系的复杂性。

那么反过来应该用继承的地方在哪里呢？一个指标是如果“想要基类的语义亚型”。Erg 由类型系统自动确定子类型的一部分（如果 Int 大于或等于 e.g.0，则为 Nat）。但是，例如，仅依靠 Erg 类型系统来创建“表示有效电子邮件地址的字符串类型”是很困难的。应该对普通字符串进行验证。然后，我们希望为验证通过的字符串对象添加一个“保证书”。这相当于向下转换到继承类。将下铸为<gtr=“46”/>与验证字符串是否为正确的电子邮件地址格式一一对应。


```erg
ValidMailAddressStr = Inherit Str
ValidMailAddressStr.
    init s: Str =
        validate s # mail-address validation
        Self.new s

s1 = "invalid mail address"
s2 = "foo@gmail.com"
_ = ValidMailAddressStr.init s1 # panic: invalid mail address
valid = ValidMailAddressStr.init s2
valid: ValidMailAddressStr # assurance that it is in the correct email address format
```

另一个指标是“记名的多相 = 想实现多态”的情况。例如，下面定义的过程接受任何类型为<gtr=“48”/>的对象。但显然，应用类型对象是错误的。因此，我们将参数类型设置为类<gtr=“50”/>。在这种情况下，只有<gtr=“51”/>对象和继承它的类<gtr=“52”/>对象作为参数。这样更保守，不用承担不必要的更多责任。


```erg
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

structural_greet! max # Hello, my name is Max.
structural_greet! john # Hello, my name is John.
greet! alice # Hello, my name is Alice.
greet! max # TypeError:
```

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_nst_vs_sst.md'>Next</a>
</p>
