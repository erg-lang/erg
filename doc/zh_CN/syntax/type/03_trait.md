# TRAIT

TRAIT 是一种记名类型，它将类型属性请求添加到记录类型中。它类似于 Python 中的抽象基类（Abstract Base Class，ABC），但具有代数运算能力。


```erg
Norm = Trait {.x = Int; .y = Int; .norm = Self.() -> Int}
```

TRAIT不区分属性和方法。

请注意，TRAIT只能进行声明，而不能进行实现（实现是通过以下称为修补程序的功能实现的）。你可以检查 TRAIT 是否在类中以子类型实现。


```erg
Point2D <: Norm
Point2D = Class {.x = Int; .y = Int}
Point2D.norm self = self.x**2 + self.y**2
```

未实现请求属性将导致错误。


```erg
Point2D <: Norm # TypeError: Point2D is not a subtype of Norm
Point2D = Class {.x = Int; .y = Int}
```

与结构类型一样，Trait 可以应用合并，替换和排除操作（e.g.）。这种方式形成的TRAIT称为即时TRAIT。


```erg
T = Trait {.x = Int}
U = Trait {.y = Int}
V = Trait {.x = Int; y: Int}
assert Structural(T and U) == Structural V
assert Structural(V not U) == Structural T
W = Trait {.x = Ratio}
assert Structural(W) !=  Structural(T)
assert Structural(W) == Structural(T.replace {.x = Ratio})
```

TRAIT 也是一种类型，因此也可以用于常规类型指定。


```erg
points: [Norm; 2] = [Point2D::new(1, 2), Point2D::new(3, 4)]
assert points.iter().map(x -> x.norm()).collect(Array) == [5, 25]
```

## TRAIT包容

扩展运算符允许你定义一个包含高级类型的 TRAIT 的 TRAIT。这称为TRAIT的<gtr=“21”/>。在下面的示例中，<gtr=“16”/>包含<gtr=“17”/>和<gtr=“18”/>。这对应于类中的继承（Inheritance），但不同于继承，可以通过组合<gtr=“19”/>来指定多个基本类型。根据<gtr=“20”/>排除一部分的TRAIT也OK。


```erg
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
ClosedAdd = Subsume Add(Self)
Sub R = Trait {
    .SubO = Type
    .`_-_` = Self.(R) -> O
}
ClosedSub = Subsume Sub(Self)
ClosedAddSub = Subsume ClosedAdd and ClosedSub
```

## 结构TRAIT

TRAIT可以结构化。


```erg
SAdd = Structural Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: SAdd|は省略できない
add|A <: SAdd| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

assert add(C.new(1), C.new(2)) == C.new(3)
```

记名任务不能只是实现请求方法，必须显式声明实现。不能用于<gtr=“23”/>类型的参数，因为在下面的示例中没有明确的实现声明。它必须是。


```erg
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: Add|は省略できる
add|A <: Add| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

add C.new(1), C.new(2) # TypeError: C is not subclass of Add
# hint: inherit or patch 'Add'
```

结构TRAIT可以没有这种实现的声明，但替代推理不起作用。使用时必须指定类型。

## 依赖项

TRAIT可以采取自变量。这与依赖关系相同。


```erg
Mapper T: Type = Trait {
    .MapIter = {Iterator}
    .map = Self(T).(T -> U) -> Self.MapIter U
}

# ArrayIterator <: Mapper
# ArrayIterator.MapIter == ArrayMapper
# [1, 2, 3].iter(): ArrayIterator Int
# [1, 2, 3].iter().map(x -> "{x}"): ArrayMapper Str
assert [1, 2, 3].iter().map(x -> "{x}").collect(Array) == ["1", "2", "3"]
```

## TRAIT中的覆盖

派生的 TRAIT 可以覆盖基础 TRAIT 的类型定义。在这种情况下，要覆盖的方法类型必须是基础方法类型的子类型。


```erg
# `Self.(R) -> O`は`Self.(R) -> O or Panic`の部分型
Div R, O: Type = Trait {
    .`/` = Self.(R) -> O or Panic
}
SafeDiv R, O = Subsume Div, {
    @Override
    .`/` = Self.(R) -> O
}
```

## 实现和解决 API 重复任务

实际的，<gtr=“26”/>和<gtr=“27”/>的定义是这样的。


```erg
Add R = Trait {
    .Output = Type
    .`_+_` = Self.(R) -> .Output
}
Sub R = Trait {
    .Output = Type
    .`_-_` = Self.(R) -> .Output
}
Mul R = Trait {
    .Output = Type
    .`*` = Self.(R) -> .Output
}
```

名为的变量具有重复的名称。如果要同时实现多个托盘，请指定。


```erg
P = Class {.x = Int; .y = Int}
# P|Self <: Add(P)|はP|<: Add(P)|に省略可能
P|Self <: Add(P)|.
    Output = P
    `_+_` self, other = P.new {.x = self.x + other.x; .y = self.y + other.y}
P|Self <: Mul(Int)|.
    Output = P
    `*` self, other = P.new {.x = self.x * other; .y = self.y * other}
```

以这种方式实现的重复 API 在使用时通常是类型推理的，但也可以通过使用显式类型来解决。


```erg
print! P.Output # TypeError: ambiguous type resolution
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## Appendix：Rust 与TRAIT的区别

Erg 的TRAIT忠于提出的TRAIT。为了能够进行代数运算，TRAIT没有实现，设计了必要时打补丁的设计。

<p align='center'>
    <a href='./02_basic.md'>Previous</a> | <a href='./04_class.md'>Next</a>
</p>
