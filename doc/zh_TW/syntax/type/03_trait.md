# 特質

Trait 是一種名義類型，它將類型屬性要求添加到記錄類型。
它類似于 Python 中的抽象基類 (ABC)，但區別在于能夠執行代數運算。

```python
Norm = Trait {.x = Int; .y = Int; .norm = Self.() -> Int}
```

特質不區分屬性和方法。

注意，trait 只能聲明，不能實現(實現是通過一個叫做 patching 的特性來實現的，后面會討論)。
可以通過指定部分類型來檢查特征在類中的實現。

```python
Point2D <: Norm
Point2D = Class {.x = Int; .y = Int}
Point2D.norm self = self.x**2 + self.y**2
```

Error if the required attributes are not implemented.

```python
Point2D <: Norm # 類型錯誤：Point2D 不是 Norm 的子類型
Point2D = Class {.x = Int; .y = Int}
```

特征與結構類型一樣，可以應用組合、替換和消除等操作(例如“T 和 U”)。 由此產生的特征稱為即時特征。

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
assert points.iter().map(x -> x.norm()).collect(Array) == [5, 25].
```

## 特征包含

擴展運算符 `...` 允許您將包含某個特征的特征定義為超類型。 這稱為特征的 __subsumption__。
在下面的示例中，`BinAddSub` 包含 `BinAdd` 和 `BinSub`。
這對應于類中的繼承，但與繼承不同的是，可以使用“和”組合多個基類型。 也允許被 `not` 部分排除的特征。

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

## 結構特征

特征可以結構化

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

名義特征不能簡單地通過實現請求方法來使用，而必須明確聲明已實現。
在以下示例中，`add`不能與`C`類型的參數一起使用，因為沒有明確的實現聲明。 它必須是`C = Class {i = Int}, Impl := Add`。

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

add C.new(1), C.new(2) # 類型錯誤：C 不是 Add 的子類
# 提示：繼承或修補“添加”
```

不需要為此實現聲明結構特征，但類型推斷不起作用。 使用時需要指定類型。

## 多態特征

特征可以帶參數。 這與多態類型相同。

```python
Mapper T: Type = Trait {
    .mapIter = {Iterator}
    .map = Self(T). (T -> U) -> Self.MapIter U
}

# ArrayIterator <: Mapper
# ArrayIterator.MapIter == ArrayMapper
# [1, 2, 3].iter(): ArrayIterator Int
# [1, 2, 3].iter().map(x -> "{x}"): ArrayMapper Str
assert [1, 2, 3].iter().map(x -> "{x}").collect(Array) == ["1", "2", "3"].
```

## Override特征

派生特征可以Override基本特征的類型定義。
在這種情況下，Override方法的類型必須是基方法類型的子類型。

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

## 在 API 中實現和解決重復的特征

`Add`、`Sub` 和 `Mul` 的實際定義如下所示。

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

`.Output` 重復。 如果要同時實現這些多個特征，請指定以下內容

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

以這種方式實現的重復 API 在使用時幾乎總是類型推斷，但也可以通過使用 `||` 顯式指定類型來解決。

```python
print! P.Output # 類型錯誤：不明確的類型
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## 附錄：與 Rust 特征的區別

Erg 的特征忠實于 [Sch?rli 等人] (https://www.ptidej.net/courses/ift6251/fall06/presentations/061122/061122.doc.pdf) 提出的特征。
為了允許代數運算，特征被設計為不能有方法實現目錄，但可以在必要時進行修補。

<p 對齊='中心'>
     <a href='./02_basic.md'>上一頁</a> | <a href='./04_class.md'>下一步</a>
</p>