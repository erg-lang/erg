# 部分定型

在 Erg 中，可以使用比较运算符和<gtr=“8”/>来确定类之间的包含关系。


```erg
Nat < Int
Int < Object
1.._ < Nat
{1, 2} > {1}
{=} > {x = Int}
{I: Int | I >= 1} < {I: Int | I >= 0}
```

请注意，它与运算符的含义不同。它声明左侧类是右侧类型的子类型，并且仅在编译时有意义。


```erg
C <: T # T: StructuralType
f|D <: E| ...

assert F < G
```

对于多相类型的子类型规范，例如，也可以指定<gtr=“11”/>。

## 结构类型，类类型关系

结构类型是用于实现结构定型的类型，如果结构相同，则将其视为相同的对象。


```erg
T = Structural {i = Int}
U = Structural {i = Int}

assert T == U
t: T = {i = 1}
assert t in T
assert t in U
```

相反，类是用于实现记名类型的类型，不能在结构上比较类型和实例。


```erg
C = Class {i = Int}
D = Class {i = Int}

assert C == D # TypeError: cannot compare classes
c = C.new {i = 1}
assert c in C
assert not c in D
```

## 子程序的局部类型

子程序的参数和返回值只采用单个类。也就是说，不能将结构型和trait作为函数的类型直接指定。必须使用子类型指定将其指定为“作为该类型子类型的单个类”。


```erg
# OK
f1 x, y: Int = x + y
# NG
f2 x, y: Add = x + y
# OK
# Aは何らかの具体的なクラス
f3<A <: Add> x, y: A = x + y
```

子程序的类型推论也遵循这个规则。当子程序中的变量中有未明示类型时，编译器首先检查该变量是否为某个类的实例，如果不是，则从作用域中的trait中寻找适合的变量。即使这样也找不到的话，就成为编译错误。这个错误可以通过使用结构型来消除，但是推论无名型有可能是程序员不想要的结果，所以设计成程序员明确地用来指定。

## 上传类


```erg
i: Int
i as (Int or Str)
i as (1..10)
i as {I: Int | I >= 0}
```
