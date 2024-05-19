# Trait

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/03_trait.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/03_trait.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

Traits are nominal types that add a type attribute requirement to record types.
It is similar to the Abstract Base Class (ABC) in Python, but it has the feature of being able to perform algebraic operations.

Traits are used when you want to identify different classes. Examples of builtin traits are `Eq` and `Add`.
`Eq` requires `==` to be implemented. `Add` requires the implementation of `+` (in-place).

So any class that implements these can be (partially) identified as a subtype of trait.

As an example, let's define a `Norm` trait that computes the norm (length) of a vector.

```python
Norm = Trait {.norm = (self: Self) -> Int}
```

trait不區分屬性和方法

注意，trait 只能聲明，不能實現(實現是通過一個叫做 patching 的特性來實現的，后面會討論)
可以通過指定部分類型來檢查Trait在類中的實現

```python
Point2D = Class {.x = Int; .y = Int}
Point2D|<: Norm|.
    Norm self = self.x**2 + self.y**2

Point3D = Class {.x = Int; .y = Int; .z = Int}
Point3D|<: Norm|.
    norm self = self.x**2 + self.y**2 + self.z**2
```

Since `Point2D` and `Point3D` implement `Norm`, they can be identified as types with the `.norm` method.

```python
norm x: Norm = x.norm()

assert norm(Point2D.new({x = 1; y = 2})) == 5
assert norm(Point3D.new({x = 1; y = 2; z = 3})) == 14
```

Error if the required attributes are not implemented.

```python,compile_fail
Point3D = Class {.x = Int; .y = Int; .z = Int}
Point3D|<: Norm|.
    foo self = 1
```

One of the nice things about traits is that you can define methods on them in Patch (described later).

```python
@Attach NotEqual
Eq = Trait {. `==` = (self: Self, other: Self) -> Bool}

NotEq = Patch Eq
NotEq.
    `! =` self, other = not self.`==` other
```

With the `NotEq` patch, all classes that implement `Eq` will automatically implement `!=`.

## Trait operations

Trait與結構類型一樣，可以應用組合、替換和消除等操作(例如"T 和 U")。由此產生的Trait稱為即時Trait

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

Trait 也是一種類型，因此可以用于普通類型規范

```python
points: [Norm; 2] = [Point2D::new(1, 2), Point2D::new(3, 4)]
assert points.iter().map(x -> x.norm()).collect(List) == [5, 25].
```

## Trait包含

`Subsume` 允許您將包含某個Trait的Trait定義為父類型。這稱為Trait的 __subsumption__
在下面的示例中，`BinAddSub` 包含 `BinAdd` 和 `BinSub`
這對應于類中的繼承，但與繼承不同的是，可以使用"和"組合多個基類型。也允許被 `not` 部分排除的Trait

```python
Add R = Trait {
    .Output = Type
    . `_+_` = Self.(R) -> Self.Output
}

Sub R = Trait {
    .Output = Type
    . `_-_` = Self.(R) -> Self.Output
}

BinAddSub = Subsume Add(Self) and Sub(Self)
```

## 結構Trait

Trait可以結構化

```python
SAdd = Structural Trait {
    . `_+_` = Self.(Self) -> Self
}
# |A <: SAdd| 不能省略
add|A <: SAdd| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

assert add(C.new(1), C.new(2)) == C.new(3)
```

名義Trait不能簡單地通過實現請求方法來使用，而必須明確聲明已實現
在以下示例中，`add`不能與`C`類型的參數一起使用，因為沒有明確的實現聲明。它必須是`C = Class {i = Int}, Impl := Add`

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: 添加| 可以省略
add|A <: Add| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

add C.new(1), C.new(2) # 類型錯誤: C 不是 Add 的子類
# 提示: 繼承或修補"添加"
```

不需要為此實現聲明結構Trait，但類型推斷不起作用。使用時需要指定類型

## 多態Trait

Trait可以帶參數。這與多態類型相同

```python
Mapper T: Type = Trait {
    .mapIter = {Iterator}
    .map = (self: Self, T -> U) -> Self.MapIter U
}

# ListIterator <: Mapper
# ListIterator.MapIter == ListMapper
# [1, 2, 3].iter(): ListIterator Int
# [1, 2, 3].iter().map(x -> "\{x}"): ListMapper Str
assert [1, 2, 3].iter().map(x -> "\{x}").collect(List) == ["1", "2", "3"].
```

## OverrideTrait

派生Trait可以Override基本Trait的類型定義
在這種情況下，Override方法的類型必須是基方法類型的子類型

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

## 在 API 中實現和解決重復的Trait

`Add`、`Sub` 和 `Mul` 的實際定義如下所示

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

`.Output` 重復。如果要同時實現這些多個Trait，請指定以下內容

```python
P = Class {.x = Int; .y = Int}
# P|Self <: Add(P)|可簡寫為 P|<: Add(P)|
P|Self <: Add(P)|.
    Output = P
    `_+_` self, other = P.new {.x = self.x + other.x; .y = self.y + other.y}
P|Self <: Mul(Int)|.
    Output = P
    `*` self, other = P.new {.x = self.x * other; .y = self.y * other}
```

以這種方式實現的重復 API 在使用時幾乎總是類型推斷，但也可以通過使用 `||` 顯式指定類型來解決

```python
print! P.Output # 類型錯誤: 不明確的類型
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## 附錄: 與 Rust Trait的區別

Erg 的Trait忠實于 [Sch?rli 等人] (<https://www.ptidej.net/courses/ift6251/fall06/presentations/061122/061122.doc.pdf>) 提出的Trait
為了允許代數運算，Trait被設計為不能有方法實現目錄，但可以在必要時進行修補

<p 對齊='中心'>
     <a href='./02_basic.md'>上一頁</a> | <a href='./04_class.md'>下一步</a>
</p>
