# TRAIT

TRAIT 是一種記名類型，它將類型屬性請求添加到記錄類型中。它類似於 Python 中的抽象基類（Abstract Base Class，ABC），但具有代數運算能力。


```erg
Norm = Trait {.x = Int; .y = Int; .norm = Self.() -> Int}
```

TRAIT不區分屬性和方法。

請注意，TRAIT只能進行聲明，而不能進行實現（實現是通過以下稱為修補程序的功能實現的）。你可以檢查 TRAIT 是否在類中以子類型實現。


```erg
Point2D <: Norm
Point2D = Class {.x = Int; .y = Int}
Point2D.norm self = self.x**2 + self.y**2
```

未實現請求屬性將導致錯誤。


```erg
Point2D <: Norm # TypeError: Point2D is not a subtype of Norm
Point2D = Class {.x = Int; .y = Int}
```

與結構類型一樣，Trait 可以應用合併，替換和排除操作（e.g.）。這種方式形成的TRAIT稱為即時TRAIT。


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

TRAIT 也是一種類型，因此也可以用於常規類型指定。


```erg
points: [Norm; 2] = [Point2D::new(1, 2), Point2D::new(3, 4)]
assert points.iter().map(x -> x.norm()).collect(Array) == [5, 25]
```

## TRAIT包容

擴展運算符允許你定義一個包含高級類型的 TRAIT 的 TRAIT。這稱為TRAIT的<gtr=“21”/>。在下面的示例中，<gtr=“16”/>包含<gtr=“17”/>和<gtr=“18”/>。這對應於類中的繼承（Inheritance），但不同於繼承，可以通過組合<gtr=“19”/>來指定多個基本類型。根據<gtr=“20”/>排除一部分的TRAIT也OK。


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

## 結構TRAIT

TRAIT可以結構化。


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

記名任務不能只是實現請求方法，必須顯式聲明實現。不能用於<gtr=“23”/>類型的參數，因為在下面的示例中沒有明確的實現聲明。它必須是。


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

結構TRAIT可以沒有這種實現的聲明，但替代推理不起作用。使用時必須指定類型。

## 依賴項

TRAIT可以採取自變量。這與依賴關係相同。


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

## TRAIT中的覆蓋

派生的 TRAIT 可以覆蓋基礎 TRAIT 的類型定義。在這種情況下，要覆蓋的方法類型必須是基礎方法類型的子類型。


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

## 實現和解決 API 重複任務

實際的，<gtr=“26”/>和<gtr=“27”/>的定義是這樣的。


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

名為的變量具有重複的名稱。如果要同時實現多個托盤，請指定。


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

以這種方式實現的重複 API 在使用時通常是類型推理的，但也可以通過使用顯式類型來解決。


```erg
print! P.Output # TypeError: ambiguous type resolution
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## Appendix：Rust 與TRAIT的區別

Erg 的TRAIT忠於提出的TRAIT。為了能夠進行代數運算，TRAIT沒有實現，設計了必要時打補丁的設計。

<p align='center'>
    <a href='./02_basic.md'>Previous</a> | <a href='./04_class.md'>Next</a>
</p>