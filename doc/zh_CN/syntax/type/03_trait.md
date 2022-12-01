# trait

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/03_trait.md%26commit_hash%3D14657486719a134f494e107774ac8f9d5a63f083)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/03_trait.md&commit_hash=14657486719a134f494e107774ac8f9d5a63f083)

Trait 是一种名义类型，它将类型属性要求添加到记录类型
它类似于 Python 中的抽象基类 (ABC)，但区别在于能够执行代数运算

```python
Norm = Trait {.x = Int; .y = Int; .norm = Self.() -> Int}
```

trait不区分属性和方法

注意，trait 只能声明，不能实现(实现是通过一个叫做 patching 的特性来实现的，后面会讨论)
可以通过指定部分类型来检查Trait在类中的实现

```python
Point2D <: Norm
Point2D = Class {.x = Int; .y = Int}
Point2D.norm self = self.x**2 + self.y**2
```

Error if the required attributes are not implemented.

```python
Point2D <: Norm # 类型错误: Point2D 不是 Norm 的子类型
Point2D = Class {.x = Int; .y = Int}
```

Trait与结构类型一样，可以应用组合、替换和消除等操作(例如"T 和 U")。由此产生的Trait称为即时Trait

```python
T = Trait {.x = Int}
U = Trait {.y = Int}
V = Trait {.x = Int; y: Int}
assert Structural(T and U) == Structural V
assert Structural(V not U) == Structural T
W = Trait {.x = Ratio}
assert Structural(W) ! = Structural(T)
assert Structural(W) == Structural(T.replace {.x = Ratio})
```

Trait 也是一种类型，因此可以用于普通类型规范

```python
points: [Norm; 2] = [Point2D::new(1, 2), Point2D::new(3, 4)]
assert points.iter().map(x -> x.norm()).collect(Array) == [5, 25].
```

## Trait包含

`Subsume` 允许您将包含某个Trait的Trait定义为超类型。这称为Trait的 __subsumption__
在下面的示例中，`BinAddSub` 包含 `BinAdd` 和 `BinSub`
这对应于类中的继承，但与继承不同的是，可以使用"和"组合多个基类型。也允许被 `not` 部分排除的Trait

```python
Add R = Trait {
    .AddO = Type
    . `_+_` = Self.(R) -> Self.AddO
}

Sub R = Trait {
    .SubO = Type
    . `_-_` = Self.(R) -> Self.SubO
}

BinAddSub = Subsume Add(Self) and Sub(Self)
```

## 结构Trait

Trait可以结构化

```python
SAdd = Structural Trait {
    . `_+_` = Self.(Self) -> Self
}
# |A <: SAdd| 不能省略
add|A <: SAdd| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

assert add(C.new(1), C.new(2)) == C.new(3)
```

名义Trait不能简单地通过实现请求方法来使用，而必须明确声明已实现
在以下示例中，`add`不能与`C`类型的参数一起使用，因为没有明确的实现声明。它必须是`C = Class {i = Int}, Impl := Add`

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: 添加| 可以省略
add|A <: Add| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

add C.new(1), C.new(2) # 类型错误: C 不是 Add 的子类
# 提示: 继承或修补"添加"
```

不需要为此实现声明结构Trait，但类型推断不起作用。使用时需要指定类型

## 多态Trait

Trait可以带参数。这与多态类型相同

```python
Mapper T: Type = Trait {
    .mapIter = {Iterator}
    .map = (self: Self, T -> U) -> Self.MapIter U
}

# ArrayIterator <: Mapper
# ArrayIterator.MapIter == ArrayMapper
# [1, 2, 3].iter(): ArrayIterator Int
# [1, 2, 3].iter().map(x -> "\{x}"): ArrayMapper Str
assert [1, 2, 3].iter().map(x -> "\{x}").collect(Array) == ["1", "2", "3"].
```

## OverrideTrait

派生Trait可以Override基本Trait的类型定义
在这种情况下，Override方法的类型必须是基方法类型的子类型

```python
# `Self.(R) -> O` is a subtype of ``Self.(R) -> O or Panic
Div R, O: Type = Trait {
    . `/` = Self.(R) -> O or Panic
}
SafeDiv R, O = Subsume Div, {
    @Override
    . `/` = Self.(R) -> O
}
```

## 在 API 中实现和解决重复的Trait

`Add`、`Sub` 和 `Mul` 的实际定义如下所示

```python
Add R = Trait {
    .Output = Type
    . `_+_` = Self.(R) -> .Output
}
Sub R = Trait {
    .Output = Type
    . `_-_` = Self.(R) -> .Output
}
Mul R = Trait {
    .Output = Type
    . `*` = Self.(R) -> .Output
}
```

`.Output` 重复。如果要同时实现这些多个Trait，请指定以下内容

```python
P = Class {.x = Int; .y = Int}
# P|Self <: Add(P)|可简写为 P|<: Add(P)|
P|Self <: Add(P)|.
    Output = P
    `_+_` self, other = P.new {.x = self.x + other.x; .y = self.y + other.y}
P|Self <: Mul(Int)|.
    Output = P
    `*` self, other = P.new {.x = self.x * other; .y = self.y * other}
```

以这种方式实现的重复 API 在使用时几乎总是类型推断，但也可以通过使用 `||` 显式指定类型来解决

```python
print! P.Output # 类型错误: 不明确的类型
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## 附录: 与 Rust Trait的区别

Erg 的Trait忠实于 [Schärli 等人] (https://www.ptidej.net/courses/ift6251/fall06/presentations/061122/061122.doc.pdf) 提出的Trait
为了允许代数运算，Trait被设计为不能有方法实现目录，但可以在必要时进行修补

<p 对齐='中心'>
     <a href='./02_basic.md'>上一页</a> | <a href='./04_class.md'>下一步</a>
</p>
