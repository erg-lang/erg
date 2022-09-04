# Newtype pattern

下面是 Rust 常用的 newtype 模式的 Erg 版本。

Erg 可以按如下方式定義類型別名，但僅指同一類型。


```erg
UserId = Int
```

因此，例如，即使類型的數值是 8 位正數，因為它與<gtr=“4”/>類型相同，所以可以輸入 10 或-1. 如果，-1 是可以彈的，但是 8 位數的性質僅用 Erg 的類型系統是不能表現的。

再比如設計某個數據庫的系統時，有幾類 ID。隨著 ID 類型的增加，例如用戶 ID，商品 ID，訂單 ID 等，可能會出現錯誤，即向函數傳遞不同類型的 ID。用戶 ID 和商品 ID 等即使在結構上等價，在語義上也是不同的。

newtype 模式是這種情況下的理想設計模式。


```erg
UserId = Class {id = Nat}
UserId.
    new id: Nat =
        assert id.dights().len() == 8, else: "UserId must be a positive number with length 8"
        UserId::__new__ {id;}

i = UserId.new(10000000)
print! i # <__main__.UserId object>
i + UserId.new(10000001) # TypeError: + is not implemented between `UserId` and `UserId`
```

構造函數保證了 8 位數的先決條件。由於丟失了<gtr=“7”/>的所有方法，因此必須重新定義每次所需的運算。如果重新定義的成本不相稱，最好使用繼承。相反，你可能希望使用沒有方法的特性，因此請根據具體情況選擇適當的方法。