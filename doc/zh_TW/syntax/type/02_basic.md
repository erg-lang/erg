# 類型的基本語法

## 類型指定

Erg 在之後指定變量類型，如下所示。也可以在賦值的同時進行。


```erg
i: Int # 聲明從現在開始使用的變量 i 為 Int 類型
i: Int = 1
j = 1 # type specification can be omitted
```

也可以為常規表達式指定類型。


```erg
i = 1: Int
f([1, "a"]: [Int or Str])
```

對於簡單變量賦值，大多數類型都是可選的。類型在定義子例程和類型時比簡單變量更有用。


```erg
# 參數類型說明
f x, y: Array Int = ...
T X, Y: Array Int = ...
```

注意，在上述情況下，都是<gtr=“17”/>。


```erg
# 大寫變量值必須是常量表達式
f X: Int = X
```

或者，如果你不完全需要類型參數信息，則可以使用將其省略。


```erg
g v: [T; _] = ...
```

但是，請注意，如果在指定類型的位置指定，則意味著<gtr=“20”/>。


```erg
f x: _, y: Int = x + y # TypeError: + is not implemented between Object and Int
```

## 子類型指定

除了使用（類型聲明運算符）指定類型與表達式之間的關係外，Erg 還使用<gtr=“22”/>（子類型聲明運算符）指定類型之間的關係。 <gtr=“23”/>的左邊只能是類。使用<gtr=“24”/>等比較結構類型。

它通常用於子程序或類型定義，而不是簡單的變量。


```erg
# 部分輸入參數
f X <: T = ...

# 請求屬性子類型（要求 .Iterator 屬性是 Iterator 類型的子類型）
Iterable T = Trait {
    .Iterator = {Iterator} # == {I | I <: Iterator}
    .iter = Self.() -> Self.Iterator T
    ...
}
```

還可以在定義類時指定子類型，以靜態方式檢查類是否為指定類型的子類型。


```erg
# C 類是 Show 的子類型
C = Class Object, Impl=Show
C.show self = ... # Show請求屬性
```

也可以僅在特定情況下指定子類型。


```erg
K T: Eq
K Int <: Show and Eq
K T = Class Object
K(T).
    `==` self, other = ...
K(Int).
    show self = ...
```

建議在實現結構類型時使用子類型。由於結構部分類型的特性，在實現請求屬性時，即使存在錯誤的拼貼或類型指定，也不會出現錯誤。


```erg
C = Class Object
C.shoe self = ... # Show 由於 Typo 沒有實現（它只是被認為是一種獨特的方法）
```

## 屬性定義

只能在模塊中為托盤和類定義屬性。


```erg
C = Class()
C.pub_attr = "this is public"
C::private_attr = "this is private"

c = C.new()
assert c.pub_attr == "this is public"
```

在或<gtr=“26”/>後換行並縮進的語法稱為批量定義（batch definition）。


```erg
C = Class()
C.pub1 = ...
C.pub2 = ...
C::priv1 = ...
C::priv2 = ...
# is equivalent to
C = Class()
C.
    pub1 = ...
    pub2 = ...
C::
    priv1 = ...
    priv2 = ...
```

## 鋸齒

可以為類型指定別名（別名）。這使你可以將長類型（如記錄類型）表示為短類型。


```erg
Id = Int
Point3D = {x = Int; y = Int; z = Int}
IorS = Int or Str
Vector = Array Int
```

此外，在錯誤顯示過程中，編譯器應盡可能使用複雜類型（在上面的示例中，不是第一種類型的右邊類型）的別名。

但是，每個模塊最多只能有一個別名，如果有多個別名，則會出現 warning。這意味著具有不同目的的類型應重新定義為不同的類型。它還可以防止將別名附加到已有別名的類型。


```erg
Id = Int
UserId = Int # TypeWarning: duplicate aliases: Id and UserId

Ids = Array Id
Ints = Array Int # TypeWarning: duplicate aliases: Isd and Ints

IorS = Int or Str
IorSorB = IorS or Bool
IorSorB_ = Int or Str or Bool # TypeWarning: duplicate aliases: IorSorB and IorSorB_

Point2D = {x = Int; y = Int}
Point3D = {...Point2D; z = Int}
Point = {x = Int; y = Int; z = Int} # TypeWarning: duplicate aliases: Point3D and Point
```

<p align='center'>
    <a href='./01_type_system.md'>Previous</a> | <a href='./03_trait.md'>Next</a>
</p>