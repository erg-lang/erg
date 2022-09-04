# Patch

Erg 不允許修改現有類型類。不能在類中定義額外的方法，而是專門化（specialization，將聲明為多相的類型單相化並定義專用方法的功能。C++ 等也不能使用。但是，在許多情況下，你希望將功能添加到現有類型類中，而修補程序就是實現這一目標的方法。


```erg
StrReverse = Patch Str
StrReverse.
    reverse self = self.iter().rev().collect(Str)

assert "abc".reverse() == "cba"
```

修補程序的名稱最好是要添加的主要功能的直接描述。這樣，要修補的類型（）的對象就可以使用修補方法（<gtr=“18”/>）。實際上，<gtr=“19”/>不是<gtr=“20”/>方法，而是添加到<gtr=“21”/>中的方法。

但是，修補程序方法的優先級低於記名（類）方法，因此不能覆蓋（覆蓋）現有類的方法。


```erg
StrangeInt = Patch Int
StrangeInt.
    `_+_` = Int.`_-_` # AssignError: .`_+_` is already defined in Int
```

如果要覆蓋，必須繼承類。但是，建議你定義一個具有不同名稱的方法，而不是覆蓋它。因為覆蓋有一些安全限制，不是那麼容易做到的。


```erg
StrangeInt = Inherit Int
StrangeInt.
    # オーバーライドするメソッドにはOverrideデコレータを付與する必要がある
    # さらに、Int.`_+_`に依存するIntのメソッドすべてをオーバーライドする必要がある
    @Override
    `_+_` = Super.`_-_` # OverrideError: Int.`_+_` is referenced by ..., so these method must also be overridden
```

## 選擇修補程序

可以為一種類型定義多個曲面片，也可以將它們組合在一起。


```erg
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
StrMultiReplace = Patch(Str)
StrMultiReverse.
    multi_replace self, pattern_and_targets: [(Pattern, Str)] = ...
StrToCamelCase = Patch(Str)
StrToCamelCase.
    to_camel_case self = ...
StrToKebabCase = Patch(Str)
StrToKebabCase.
    to_kebab_case self = ...

StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
```


```erg
{StrBoosterPack; ...} = import "foo"

assert "abc".reverse() == "cba"
assert "abc".multi_replace([("a", "A"), ("b", "B")]) == "ABc"
assert "to camel case".to_camel_case() == "toCamelCase"
assert "to kebab case".to_kebab_case() == "to-kebab-case"
```

如果可以定義多個修補程序，某些修補程序可能會導致重複的實現。


```erg
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
# more efficient implementation
StrReverseMk2 = Patch(Str)
StrReverseMk2.
    reverse self = ...

"hello".reverse() # PatchSelectionError: multiple choices of `.reverse`: StrReverse, StrReverseMk2
```

在這種情況下，可以使用相關函數格式而不是方法格式來實現唯一性。


```erg
assert StrReverseMk2.reverse("hello") == "olleh"
```

也可以通過選擇性導入來實現唯一性。


```erg
{StrReverseMk2; ...} = import "foo"

assert StrReverseMk2.reverse("hello") == "olleh"
```

## 粘合面片（Glue Patch）

修補程序還可以關聯類型。將<gtr=“23”/>與<gtr=“24”/>關聯起來。這些面片稱為“粘合面片”（Glue Patch）。由於<gtr=“25”/>是一個內置類型，因此用戶需要一個粘合貼片來改裝托盤。


```erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch Str, Impl := Reverse
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

只能為一對類型和托盤定義一個粘合曲面片。這是因為，如果多個粘合貼片同時“可見”，則無法唯一確定選擇哪個實現。但是，你可以在切換到其他範圍（模塊）時替換修補程序。


```erg
NumericStr = Inherit Str
NumericStr.
    ...

NumStrRev = Patch NumericStr, Impl := Reverse
NumStrRev.
    ...
# DuplicatePatchError: NumericStr is already associated with `Reverse`
# hint: `Str` (superclass of `NumericStr`) is associated with `Reverse` by `StrReverse`
```

## Appendix：Rust 與trait的關係

Erg 修補程序相當於 Rust 的 impl 塊（後置）。


```rust
// Rust
trait Reverse {
    fn reverse(self) -> Self;
}

impl Reverse for String {
    fn reverse(self) -> Self {
        self.chars().rev().collect()
    }
}
```

可以說，Rust Traitt 是 Erg Traitt 和補丁的功能的結合。這樣說來，Rust 的trait聽起來更方便，其實也不盡然。


```erg
# Erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch(Str, Impl := Reverse)
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

Erg 將 impl 塊對象化為修補程序，以便在從其他模塊導入時進行選擇性導入。此外，還允許在外部結構中實現外部托盤。此外，由於結構類型的不同，也不需要 dyn trait 和 impl trait 語法。


```erg
# Erg
reversible: [Reverse; 2] = [[1, 2, 3], "hello"]

iter|T|(i: Iterable T): Iterator T = i.iter()
```


```rust
// Rust
let reversible: [Box<dyn Reverse>; 2] = [Box::new([1, 2, 3]), Box::new("hello")];

fn iter<I>(i: I) -> impl Iterator<Item = I::Item> where I: IntoIterator {
    i.into_iter()
}
```

## 全稱補丁

你可以為特定類型定義修補程序，也可以為“常規函數類型”等定義修補程序。在這種情況下，將想要給出自由度的項作為參數（在下面的情況下為）。以這種方式定義的曲面片稱為全稱曲面片。正如你所看到的，全稱修補程序是一個返回修補程序的函數，但它本身也可以被視為修補程序。


```erg
FnType T: Type = Patch(T -> T)
FnType(T).
    type = T

assert (Int -> Int).type == Int
```

## 結構補丁

此外，還可以為滿足某一結構的所有類型定義修補程序。但是，它的優先級低於記名修補程序和類方法。

在定義結構修補程序時，請仔細設計，因為擴展可能會導致不成立，如下所示。


```erg
# これはStructuralにするべきではない
Norm = Structural Patch {x = Int; y = Int}
Norm.
    norm self = self::x**2 + self::y**2

Point2D = Class {x = Int; y = Int}
assert Point2D.new({x = 1; y = 2}).norm() == 5

Point3D = Class {x = Int; y = Int; z = Int}
assert Point3D.new({x = 1; y = 2; z = 3}).norm() == 14 # AssertionError:
```

<p align='center'>
    <a href='./06_nst_vs_sst.md'>Previous</a> | <a href='./08_value.md'>Next</a>
</p>