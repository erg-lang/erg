# Class

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/04_class.md%26commit_hash%3D157f51ae0e8cf3ceb45632b537ebe3560a5500b7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/04_class.md&commit_hash=157f51ae0e8cf3ceb45632b537ebe3560a5500b7)

Erg 中的类大致是一种可以创建自己的元素(实例)的类型。
这是一个简单类的示例。

```python
Person = Class {.name = Str; .age = Nat}
# 如果 `.new` 没有定义，那么 Erg 将创建 `Person.new = Person::__new__`
Person.
    new name, age = Self::__new__ {.name = name; .age = age}

john = Person.new "John Smith", 25
print! john # <Person object>
print! classof(john) # Person
```

赋予"Class"的类型(通常是记录类型)称为需求类型(在本例中为"{.name = Str; .age = Nat}")。
可以使用 `<Class name>::__new__ {<attribute name> = <value>; 创建实例 ...}` 可以创建。
`{.name = "约翰·史密斯"; .age = 25}` 只是一条记录，但它通过传递 `Person.new` 转换为 `Person` 实例。
创建此类实例的子例程称为构造函数。
在上面的类中，`.new` 方法被定义为可以省略字段名等。

请注意，以下不带换行符的定义将导致语法错误。

```python
Person.new name, age = ... # 语法错误：不能直接在对象上定义属性
```

> __Warning__：这是最近添加的规范，后续文档中可能不会遵循。 如果你发现它，请报告它。

## 实例和类属性

在 Python 和其他语言中，实例属性通常在块侧定义如下，但请注意，这样的写法在 Erg 中具有不同的含义。

```python
# Python
class Person:
    name: str
    age: int
```

```python
# 在Erg中，这个符号意味着类属性的声明(不是实例属性)
Person = Class()
Person.
    name: Str
    age: Int
```

```python
# 以上 Python 代码的 Erg 代码
Person = Class {
    .name = Str
    .age = Nat
}
```

元素属性(在记录中定义的属性)和类型属性(也称为实例/类属性，尤其是在类的情况下)是完全不同的东西。 类型属性是类型本身的属性。 当一个类型的元素本身没有所需的属性时，它指的是一个类型属性。 元素属性是元素直接拥有的唯一属性。
为什么要进行这种区分? 如果所有属性都是元素属性，那么在创建对象时复制和初始化所有属性将是低效的。
此外，以这种方式划分属性明确了诸如"该属性是共享的"和"该属性是分开持有的"之类的角色。

下面的例子说明了这一点。 `species` 属性对所有实例都是通用的，因此将其用作类属性更自然。 但是，属性 `name` 应该是实例属性，因为每个实例都应该单独拥有它。

```python
Person = Class {name = Str}
Person::
    species = "human"
Person.
    describe() =
        log "species: {species}"
    greet self =
        log "Hello, My name is {self::name}."

Person.describe() # 类型：Person
Person.greet() # 类型错误: 未绑定的方法 Person.greet 需要一个参数

john = Person.new {name = "John"}
john.describe() # 类型: human
john.greet() # 你好，我是约翰

alice = Person.new {name = "Alice"}
alice.describe() # 类型: human
alice.greet() # 你好，我是爱丽丝
```

顺便说一下，如果实例属性和类型属性具有相同的名称和相同的类型，则会发生编译错误。 这是为了避免混淆。

```python
C = Class {.i = Int}
C.i = 1 # 属性错误：`.i` 已在实例字段中定义
```

## 类(Class), 类型(Type)

请注意，`1` 的类和类型是不同的。
只有一个类 `Int` 是 `1` 的生成器。 可以通过`classof(obj)`或`obj.__class__`获取对象所属的类。
相比之下，`1`有无数种。 例如，`{1}, {0, 1}, 0..12, Nat, Int, Num`。
但是，可以将最小类型定义为单一类型，在本例中为"{1}"。 可以通过`Typeof(obj)`获取对象所属的类型。 这是一个编译时函数。
对象可以使用补丁方法以及类方法。
Erg 不允许您添加类方法，但您可以使用 [patch](./07_patch.md) 来扩展类。

您还可以从现有类([Inheritable](../29_decorator.md#可继承) 类)继承。
您可以使用 `Inherit` 创建一个继承类。 左侧的类型称为派生类，右侧的"继承"的参数类型称为基类(继承类)。

```python
MyStr = Inherit Str
# other: 如果你设置 ``other: Str''，你可以使用 MyStr。
MyStr.
    `-` self, other: Str = self.replace other, ""

abc = MyStr.new("abc")
# 这里的比较是向上的
assert abc - "b" == "ac"
```

与 Python 不同，默认情况下，定义的 Erg 类是 `final`(不可继承的)。
要使类可继承，必须将 `Inheritable` 装饰器附加到该类。
Str` 是可继承的类之一。

```python
MyStr = Inherit Str # OK
MyStr2 = Inherit MyStr # NG

@Inheritable
InheritableMyStr = Inherit Str
MyStr3 = Inherit InheritableMyStr # OK
```

`Inherit Object` 和 `Class()` 在实践中几乎是等价的。 一般使用后者。

类具有与类型不同的等价检查机制。
类型基于其结构进行等效性测试。

```python
Person = {.name = Str; .age = Nat}
Human = {.name = Str; .age = Nat}

assert Person == Human
```

class has no equivalence relation defined.

```python
Person = Class {.name = Str; .age = Nat}
Human = Class {.name = Str; .age = Nat}

Person == Human # 类型错误：无法比较类
```

## 与结构类型的区别

我们说过类是一种可以生成自己的元素的类型，但这并不是严格的描述。 事实上，一个记录类型+补丁可以做同样的事情。

```python
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    new name, age = {.name; .age}

john = Person.new("John Smith", 25)
```

使用类有四个优点。
第一个是构造函数经过有效性检查，第二个是它的性能更高，第三个是您可以使用符号子类型(NST)，第四个是您可以继承和覆盖。

我们之前看到记录类型 + 补丁也可以定义一个构造函数(某种意义上)，但这当然不是一个合法的构造函数。 这当然不是一个合法的构造函数，因为它可以返回一个完全不相关的对象，即使它调用自己`.new`。 在类的情况下，`.new` 被静态检查以查看它是否生成满足要求的对象。

~

类的类型检查只是检查对象的`。 __class__` 对象的属性。 因此可以快速检查一个对象是否属于一个类型。

~

Erg 在课堂上启用 NST； NST 的优点包括健壮性。
在编写大型程序时，经常会出现对象的结构巧合匹配的情况。

```python
Dog = {.name = Str; .age = Nat}
DogImpl = Patch Dog
DogImpl.
    bark = log "Yelp!"
...
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    greet self = log "Hello, my name is {self.name}."

john = {.name = "John Smith"; .age = 20}
john.bark() # "Yelp!"
```

`Dog` 和 `Person` 的结构完全一样，但让动物打招呼，让人类吠叫显然是无稽之谈。
前者是不可能的，所以让它不适用更安全。 在这种情况下，最好使用类。

```python
Dog = Class {.name = Str; .age = Nat}
Dog.bark = log "Yelp!"
...
Person = Class {.name = Str; .age = Nat}
Person.greet self = log "Hello, my name is {self.name}."

john = Person.new {.name = "John Smith"; .age = 20}
john.bark() # 类型错误: `Person` 对象没有方法 `.bark`。
```

另一个特点是补丁添加的类型属性是虚拟的，实现类不作为实体保存。
也就是说，`T.x`、`T.bar` 是可以通过与 `{i = Int}` 兼容的类型访问(编译时绑定)的对象，并且未在 `{i = Int}` 或 ` C`。
相反，类属性由类本身持有。 因此，它们不能被不处于继承关系的类访问，即使它们具有相同的结构。

```python
C = Class {i = Int}
C.
    foo self = ...
print! dir(C) # ["foo", ...].

T = Patch {i = Int}
T.
    x = 1
    bar self = ...
print! dir(T) # ["bar", "x", ...].
assert T.x == 1
assert {i = 1}.x == 1
print! T.bar # <函数 bar>
{i = Int}.bar # 类型错误：Record({i = Int}) 没有方法 `.bar`。
C.bar # 类型错误：C 没有方法 `.bar` 打印！
print! {i = 1}.bar # <方法 bar>
C.new({i = 1}).bar # <方法 bar>
```

## 与数据类的区别

有两种类型的类：常规类，通过`Class`成为记录类，以及从记录类继承(`Inherit`)的数据类。
数据类继承了记录类的功能，具有分解赋值、默认实现的`==`和`hash`等特性。另一方面，数据类有自己的等价关系和格式展示。
另一方面，如果要定义自己的等价关系或格式显示，则应使用普通类。

```python
C = Class {i = Int}
c = C.new {i = 1}
d = C.new {i = 2}
print! c # <C object>
c == d # 类型错误：`==` 没有为 `C` 实现

D = Inherit {i = Int}
e = D::{i = 1} # 与`e = D.new {i = 1}`相同
f = D::{i = 2}
print! e # D(i=1)
assert e ! = f
```

## 枚举类

为了便于定义"Or"类型的类，提供了一个"Enum"。

```python
X = Class()
Y = Class()
XorY = Enum X, Y
```

每种类型都可以通过`XorY.X`、`XorY.Y`来访问，构造函数可以通过`X.new |> XorY.new`获得。

```python
x1 = XorY.new X.new()
x2 = (X.new |> XorY.new)()
x3 = (Y.new |> XorY.new)()
assert x1 == x2
assert x1 != x3
```

## 类关系

类是需求类型的子类型。 类中可以使用需求类型的方法(包括补丁方法)。

```python
T = Trait {.foo = Foo}
C = Class(... , impl: T)
C.
    foo = foo
    bar x = ...
assert C < T
assert C.foo == foo
assert not T < C
assert T.foo == Foo
```

<p align='center'>
    <a href='./03_trait.md'>上一页</a> | <a href='./05_inheritance.md'>下一页</a>
</p>
