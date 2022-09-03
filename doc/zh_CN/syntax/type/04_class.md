# Class

Erg 中的类通常可以生成其自身的元素（实例）。下面是一个简单类的示例。


```erg
Person = Class {.name = Str; .age = Nat}
# .newが定義されなかった場合、自動で`Person.new = Person::__new__`となります
Person.
    new name, age = Self::__new__ {.name = name; .age = age}

john = Person.new "John Smith", 25
print! john # <Person object>
print! classof(john) # Person
```

给出的类型（通常为记录）称为要求类型（在本例中为<gtr=“21”/>）。可以在<gtr=“22”/>中生成实例。<gtr=“23”/>只是一条记录，但它通过<gtr=“24”/>转换为<gtr=“25”/>实例。生成此类实例的子例程称为构造函数。上面的类定义了<gtr=“26”/>方法，以便可以省略字段名等。

请注意，如下所示不换行定义会导致语法错误。


```erg
Person.new name, age = ... # SyntaxError: cannot define attributes directly on an object
```

> ：这是最近添加的规范，在以后的文档中可能不受保护。如果发现就报告。

## 实例属性、类属性

在 Python 和其他语言中，很多情况下都是在块端定义实例属性，如下所示，这种写法在 Erg 中是另外一个意思，需要注意。


```python
# Python
class Person:
    name: str
    age: int
```


```erg
# Ergでこの書き方はクラス属性の宣言を意味する(インスタンス属性ではない)
Person = Class()
Person.
    name: Str
    age: Int
```


```erg
# 上のPythonコードに対応するErgコード
Person = Class {
    .name = Str
    .age = Nat
}
```

元素属性（在记录中定义的属性）和类型属性（在类中特别称为实例属性/类属性）是完全不同的。类型属性是类型本身所具有的属性。类型的要素在自身中没有目标属性时参照类型属性。要素属性是要素直接具有的固有属性。为什么要做这样的划分？如果全部都是要素属性，则在生成对象时需要复制、初始化所有属性，这是因为效率低下。另外，这样分开的话，“这个属性是共用的”“这个属性是分开拥有的”等作用就会明确。

用下面的例子来说明。由于这一属性在所有实例中都是共通的，所以作为类属性更为自然。但是，由于这一属性应该是各个实例各自持有的，所以应该是实例属性。


```erg
Person = Class {name = Str}
Person::
    species = "human"
Person.
    describe() =
        log "species: {species}"
    greet self =
        log "Hello, My name is {self::name}."

Person.describe() # species: human
Person.greet() # TypeError: unbound method Person.greet needs an argument

john = Person.new {name = "John"}
john.describe() # species: human
john.greet() # Hello, My name is John.

alice = Person.new {name = "Alice"}
alice.describe() # species: human
alice.greet() # Hello, My name is Alice.
```

顺便一提，如果实例属性和类型属性中存在同名、同类型的属性，就会出现编译错误。这是为了避免混乱。


```erg
C = Class {.i = Int}
C.
    i = 1 # AttributeError: `.i` is already defined in instance fields
```

## Class, Type

请注意，类类型与不同。只有一个类可以从中生成<gtr=“31”/>。可以使用<gtr=“33”/>或<gtr=“34”/>获取对象所属的类。与此相对，<gtr=“35”/>有无数个类型。例如，<gtr=“36”/>。但是，最小的类型可以是一个，在这种情况下是<gtr=“37”/>。可以通过<gtr=“38”/>获取对象的类型。这是一个编译时函数，顾名思义，它是在编译时计算的。除了类方法外，对象还可以使用修补程序方法。Erg 不能添加类方法，但可以使用<gtr=“39”/>进行扩展。

也可以继承现有的类（对于类）。<gtr=“40”/>表示继承。左边的类型称为派生类，右边的<gtr=“41”/>参数类型称为基类。


```erg
MyStr = Inherit Str
# other: StrとしておけばMyStrでもOK
MyStr.
    `-` self, other: Str = self.replace other, ""

abc = MyStr.new("abc")
# ここの比較はアップキャストが入る
assert abc - "b" == "ac"
```

与 Python 不同，定义的 Erg 类缺省为（不可继承）。要使其可继承，必须为类指定<gtr=“44”/>装饰器。<gtr=“45”/>是可继承类之一。


```erg
MyStr = Inherit Str # OK
MyStr2 = Inherit MyStr # NG

@Inheritable
InheritableMyStr = Inherit Str
MyStr3 = Inherit InheritableMyStr # OK
```

和<gtr=“47”/>在实际应用中大致等效。一般使用后者。

类的等价机制不同于类型。类型根据结构确定等价性。


```erg
Person = {.name = Str; .age = Nat}
Human = {.name = Str; .age = Nat}

assert Person == Human
```

类没有定义等价关系。


```erg
Person = Class {.name = Str; .age = Nat}
Human = Class {.name = Str; .age = Nat}

Person == Human # TypeError: cannot compare classes
```

## 与结构类型的区别

类是一种可以生成自己元素的类型，但这并不是一个严格的描述。因为实际上，记录类型 + 修补程序也可以做到这一点。


```erg
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    new name, age = {.name; .age}

john = Person.new("John Smith", 25)
```

使用类有四个好处。一是检查构造函数的合法性，二是性能高，三是可以使用记名部分类型 (NST)，四是可以继承和覆盖。

我们已经看到记录类型 + 修补程序也可以定义构造函数（类似），但这当然不是合法的构造函数。因为你可以返回一个自称但完全不相关的对象。对于类，将静态检查是否生成满足要求的对象。

~

类类型检查只需查看对象的属性即可完成。因此，检查对象是否属于该类型的速度较快。

~

Erg 在类中提供了 NST。NST 的优点包括强健性。在编写大型程序时，对象的结构仍然会偶然匹配。


```erg
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

虽然和的结构完全相同，但允许动物打招呼和人类吠叫显然是无稽之谈。且不说后者，让前者不适用更安全，因为前者是不可能的。在这种情况下，最好使用类。


```erg
Dog = Class {.name = Str; .age = Nat}
Dog.
    bark = log "Yelp!"
...
Person = Class {.name = Str; .age = Nat}
Person.
    greet self = log "Hello, my name is {self.name}."

john = Person.new {.name = "John Smith"; .age = 20}
john.bark() # TypeError: `Person` object has no method `.bark`
```

另一个特征是，通过修补程序添加的类型属性是虚拟的，而不是作为实体保存在要实现的类中。也就是说，和<gtr=“54”/>是与<gtr=“55”/>兼容的类型可以访问（在编译时绑定）的对象，而不是在<gtr=“56”/>或<gtr=“57”/>中定义的对象。相反，类属性由类自己维护。因此，结构相同但不具有继承关系的类无法访问。


```erg
C = Class {i = Int}
C.
    foo self = ...
print! dir(C) # ["foo", ...]

T = Patch {i = Int}
T.
    x = 1
    bar self = ...
print! dir(T) # ["bar", "x", ...]
assert T.x == 1
assert {i = 1}.x == 1
print! T.bar # <function bar>
{i = Int}.bar # TypeError: Record({i = Int}) has no method `.bar`
C.bar # TypeError: C has no method `.bar`
print! {i = 1}.bar # <method bar>
print! C.new({i = 1}).bar # <method bar>
```

## 与数据类的区别

类可以是通过请求记录的常规类，也可以是继承记录（<gtr=“59”/>）的数据类。数据类继承了记录的功能，可以分解赋值，缺省情况下实现<gtr=“60”/>和<gtr=“61”/>。相反，如果你想定义自己的等价关系和格式显示，则可以使用常规类。


```erg
C = Class {i = Int}
c = C.new {i = 1}
d = C.new {i = 2}
print! c # <C object>
c == d # TypeError: `==` is not implemented for `C`

D = Inherit {i = Int}
e = D::{i = 1} # e = D.new {i = 1}と同じ
f = D::{i = 2}
print! e # D(i = 1)
assert e != f
```

## Enum Class

提供以帮助定义 Or 类型的类。


```erg
X = Class()
Y = Class()
XorY = Enum X, Y
```

每种类型都可以按和<gtr=“64”/>进行访问，构造函数可以按<gtr=“65”/>进行检索。是接收类并返回其构造函数的方法。


```erg
x1 = XorY.new X.new()
x2 = XorY.cons(X)()
assert x1 == x2
```

## 包含关系

类是需求类型的子类型。你可以使用要求类型的方法（包括修补程序方法）。


```erg
T = Trait {.foo = Foo}
C = Class(..., Impl: T)
C.
    foo = foo
    bar x = ...
assert C < T
assert C.foo == foo
assert not T < C
assert T.foo == Foo
```

<p align='center'>
    <a href='./03_trait.md'>Previous</a> | <a href='./05_inheritance.md'>Next</a>
</p>