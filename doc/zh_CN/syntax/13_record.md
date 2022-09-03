# 记录

记录是一个集合，它具有通过键访问的 Dict 和在编译时检查访问的元组的性质。如果你使用过 JavaScript，请将其视为对象文字符号（更高级）。


```erg
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # Error: john is not subscribable
```

和<gtr=“15”/>部分称为属性，<gtr=“16”/>和<gtr=“17”/>部分称为属性值。

它与 JavaScript 对象文字的区别在于不能以字符串形式访问。也就是说，属性不仅仅是字符串。这可能是因为它在编译时决定对值的访问，也可能是因为字典和记录是不同的。也就是说，是 Dict，<gtr=“19”/>是记录。那么，词典和记录该如何区分使用呢？通常建议使用记录。记录具有以下优点：编译时检查元素是否存在，并且可以指定<gtr=“21”/>。可见性规范相当于 public/private 规范，例如在 Java 语言中。有关详细信息，请参见<gtr=“20”/>。


```erg
a = {x = 1; .y = x + 1}
a.x # AttributeError: x is private
# Hint: declare as `.x`
assert a.y == 2
```

对于熟悉 JavaScript 的人来说，上面的例子可能很奇怪，但如果简单地声明，则外部无法访问，如果加上<gtr=“23”/>，则可以通过<gtr=“24”/>访问。

还可以显式指定属性的类型。


```erg
anonymous = {
    .name: Option! Str = !None
    .age = 20
}
anonymous.name.set! "John"
```

记录也可以有方法。


```erg
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

关于记录有一个值得注意的语法。当记录的所有属性值都是类（结构类型不允许）时，记录本身将其属性视为请求属性。这种类型称为记录类型。有关详细信息，请参阅记录部分。


```erg
# レコード
john = {.name = "John"}
# レコード型
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am {n.name}"
greet! john # "Hello, I am John"

print! Named.name # Str
```

## 分解记录

可以按如下方式分解记录。


```erg
record = {x = 1; y = 2}
{x = a; y = b} = record
assert a == 1
assert b == 2

point = {x = 2; y = 3; z = 4}
match point:
    {x = 0; y = 0; z = 0} -> "origin"
    {x = _; y = 0; z = 0} -> "on the x axis"
    {x = 0; ...} -> "x = 0"
    {x = x; y = y; z = z} -> "({x}, {y}, {z})"
```

此外，如果记录具有与属性同名的变量，则可以将或<gtr=“26”/>省略为<gtr=“27”/>，将<gtr=“28”/>或<gtr=“29”/>省略为<gtr=“30”/>。但是，如果只有一个属性，则必须使用<gtr=“31”/>将其与集合区分开来。


```erg
x = 1
y = 2
xy = {x; y}
a = 1
b = 2
ab = {.a; .b}
assert ab.a == 1
assert ab.b == 2

record = {x;}
tuple = {x}
assert tuple.1 == 1
```

此语法可用于分解记录并将其赋给变量。


```erg
# same as `{x = x; y = y} = xy`
{x; y} = xy
assert x == 1
assert y == 2
# same as `{.a = a; .b = b} = ab`
{a; b} = ab
assert a == 1
assert b == 2
```

## 空记录

空记录由表示。与 Unit 一样，空记录也是其类本身。


```erg
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

空记录不同于空 Dict或空集<gtr=“34”/>。尤其要注意它与<gtr=“35”/>的含义正好相反（在 Python 中，<gtr=“36”/>是一个空字典，而在 Erg 中，它是<gtr=“37”/>）。作为枚举类型，<gtr=“38”/>是空类型，不包含任何元素。类型是对其进行的类化。相反，记录类中的<gtr=“40”/>没有请求实例属性，因此所有对象都是它的元素。是此别名。<gtr=“42”/>（修补程序）具有非常基本的提供方法，如<gtr=“43”/>。


```erg
AnyPatch = Patch Structural {=}
    .__sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

请注意，没有其他类型和类在结构上与，<gtr=“45”/>类型等效，如果用户定义类型时在右边指定<gtr=“46”/>，<gtr=“47”/>，则会出错。这可以防止将<gtr=“48”/>转换为<gtr=“49”/>的错误。此外，如果定义组合结果为<gtr=“50”/>的类型（例如<gtr=“51”/>），则会发出警告，将其简单地定义为<gtr=“52”/>。

## 即时块

Erg 还有一个语法叫即时块，它只是返回最后评估的值。不能保留属性。


```erg
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # SyntaxError: cannot define an attribute in an entity block
```

## 数据类

如果尝试单独实现方法，则必须直接在实例中定义原始记录（由记录文本生成的记录）。这效率很低，而且随着属性数量的增加，错误显示等很难看到，也很难使用。


```erg
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
john + 1
# TypeError: + is not implemented for {name = Str; age = Int; .greet! = Ref(Self).() => None; inc_age! = Ref!(Self).() => None}, Int
```

因此，在这种情况下，我们将继承记录类。此类类称为数据类。我们将在部分详细讨论这一点。


```erg
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
john + 1
# TypeError: + is not implemented for Person, Int
```

<p align='center'>
    <a href='./12_dict.md'>Previous</a> | <a href='./14_set.md'>Next</a>
</p>
