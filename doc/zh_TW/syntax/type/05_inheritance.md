# 繼承（Inheritance）

通過繼承，你可以定義一個新類，該新類將添加或特定於現有類。繼承類似於trait中的包容。繼承的類將成為原始類的子類型。


```erg
NewInt = Inherit Int
NewInt.
    plus1 self = self + 1

assert NewInt.new(1).plus1() == 2
assert NewInt.new(1) + NewInt.new(1) == 2
```

如果你希望新定義的類是可繼承類，則必須指定裝飾器。

可選參數允許你具有其他實例屬性。但是，不能為值類添加實例屬性。


```erg
@Inheritable
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}

john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

MailAddress = Inherit Str, additional: {owner = Str} # TypeError: instance variables cannot be added to a value class
```

Erg 中例外的是不能繼承型的設計。因為<gtr=“17”/>是絕對不能生成實例的特殊類。

## 枚舉類繼承

也可以繼承以為類的枚舉類。在這種情況下，你可以通過指定選項參數<gtr=“18”/>來刪除任何選項（使用<gtr=“19”/>可以選擇多個選項）。仍不能添加。添加選擇的類不是原始類的子類型。


```erg
Number = Class Int or Float or Complex
Number.
    abs(self): Float =
        match self:
            i: Int -> i.abs().into Float
            f: Float -> f.abs()
            c: Complex -> c.abs().into Float

# matchの選択肢でc: Complexは現れ得ない
RealNumber = Inherit Number, Excluding: Complex
```

同樣，也可以指定。


```erg
Months = Class 0..12
MonthsNot31Days = Inherit Months, Excluding: {1, 3, 5, 7, 8, 10, 12}

StrMoreThan3 = Class StrWithLen N | N >= 3
StrMoreThan4 = Inherit StrMoreThan3, Excluding: StrWithLen N | N == 3
```

## 覆蓋

與修補程序相同，你可以在原始類型中添加新方法，但可以進一步“覆蓋”類。覆蓋稱為覆蓋。覆蓋必須滿足三個條件。首先，默認情況下，覆蓋是錯誤的，因此必須添加裝飾器。此外，覆蓋不能更改方法類型。必須是原始類型的子類型。如果要覆蓋其他方法引用的方法，則必須覆蓋所有引用的方法。

為什麼要有這樣的條件呢？這是因為覆蓋不僅可以改變一個方法的行為，還可以影響另一個方法的行為。

首先，從第一個條件開始解說。這是為了防止“意外覆蓋”。這意味著必須在裝飾器中顯示，以防止派生類中新定義的方法的名稱碰巧與基類衝突。

接下來，我們考慮第二個條件。這是為了保持類型的完整性。派生類是基類的子類型，因此其行為也必須與基類兼容。

最後，考慮第三個條件。這個條件是 Erg 特有的，在其他面向對象的語言中並不常見，但這也是為了安全起見。看看沒有這個的時候會發生什麼不好的事情。


```erg
# Bad example
@Inheritable
Base! = Class {x = Int!}
Base!.
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!.
    @Override
    g! ref! self = self.f!() # InfiniteRecursionWarning: This code falls into an infinite loop
    # OverrideError: method `.g` is referenced by `.f` but not overridden
```

繼承類覆蓋<gtr=“25”/>方法並將處理轉發到<gtr=“26”/>。但是，基類的<gtr=“27”/>方法將其處理轉發到<gtr=“28”/>，從而導致無限循環。 <gtr=“29”/>在<gtr=“30”/>類中是一個沒有問題的方法，但由於被覆蓋而被意外地使用，並被破壞。

因此，通常需要重寫所有可能受覆蓋影響的方法。 Erg 將這一規則納入規範。


```erg
# OK
@Inheritable
Base! = Class {x = Int!}
Base!.
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!.
    @Override
    f! ref! self =
        print! self::x
        self::x.update! x -> x + 1
    @Override
    g! ref! self = self.f!()
```

但這一規範並不能完全解決覆蓋問題。編譯器無法檢測覆蓋是否修復了問題。創建派生類的程序員有責任修改替代的影響。應盡可能定義別名方法。

### 替換trait（類似於）

你不能在繼承過程中替換 TRAIT，但有一個示例似乎是這樣做的。

例如，（實現<gtr=“32”/>）的子類型<gtr=“33”/>似乎正在重新實現<gtr=“34”/>。


```erg
Int = Class ..., Impl := Add() and ...
```

但實際上，中的<gtr=“36”/>是<gtr=“37”/>的縮寫，<gtr=“38”/>只是用<gtr=“39”/>覆蓋。兩者是不同的trait（<gtr=“40”/>是<gtr=“42”/>，因此<gtr=“41”/>）。

## 禁止多重繼承

Erg 不允許常規類之間的 Intersection、Diff 或 Complement。


```erg
Int and Str # TypeError: cannot unite classes
```

此規則不允許繼承多個類，即多重繼承。


```erg
IntAndStr = Inherit Int and Str # SyntaxError: multiple inheritance of classes is not allowed
```

但是，可以使用 Python 多重繼承類。

## 禁止多層繼承

Erg 繼承也禁止多層繼承。也就是說，你不能定義繼承的類，也不能定義繼承的類。但是，繼承的（Inheritable）類除外。

此外，Python 多層繼承類仍然可用。

## 禁止改寫源屬性

Erg 無法重寫源屬性。這有兩個意思。首先，對繼承的類屬性執行更新操作。不僅不能重新賦值，也不能通過方法進行更新。

覆蓋與重寫不同，因為它是一種使用更特定的方法進行覆蓋的操作。替代也必須使用兼容類型進行替換。


```erg
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!.
    var = !1
    inc_pub! ref! self = self.pub.update! p -> p + 1

Inherited! = Inherit Base!:
Inherited!.
    var.update! v -> v + 1
    # TypeError: can't update base class variables
    @Override
    inc_pub! ref! self = self.pub + 1
    # OverrideError: `.inc_pub!` must be subtype of `Self!.() => ()`
```

第二種是對從其繼承的（可變）實例屬性執行更新操作。這也是禁止的。只能從基類提供的方法更新基類的實例屬性。無論屬性的可視性如何，都不能直接更新。但是可以讀取。


```erg
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!.
    inc_pub! ref! self = self.pub.update! p -> p + 1
    inc_pri! ref! self = self::pri.update! p -> p + 1

Inherited! = Inherit Base!:
Inherited!.
    # OK
    add2_pub! ref! self =
        self.inc_pub!()
        self.inc_pub!()
    # NG, `Child` cannot touch `self.pub` and `self::pri`
    add2_pub! ref! self =
        self.pub.update! p -> p + 2
```

最後，Erg 只能繼承添加新屬性和覆蓋基類方法。

## 繼承用法

如果正確使用，繼承是一個強大的功能，但另一方面，它也有一個缺點，即類之間的依賴關係容易變得複雜，特別是在使用多重繼承和多層繼承時，這種趨勢更為明顯。依賴項的複雜性可能會降低代碼的可維護性。 Erg 禁止多重繼承和多層繼承是為了降低這種風險，而引入類修補功能是為了在繼承“添加功能”的同時減少依賴關係的複雜性。

那麼反過來應該用繼承的地方在哪裡呢？一個指標是如果“想要基類的語義亞型”。 Erg 由類型系統自動確定子類型的一部分（如果 Int 大於或等於 e.g.0，則為 Nat）。但是，例如，僅依靠 Erg 類型系統來創建“表示有效電子郵件地址的字符串類型”是很困難的。應該對普通字符串進行驗證。然後，我們希望為驗證通過的字符串對象添加一個“保證書”。這相當於向下轉換到繼承類。將下鑄為<gtr=“46”/>與驗證字符串是否為正確的電子郵件地址格式一一對應。


```erg
ValidMailAddressStr = Inherit Str
ValidMailAddressStr.
    init s: Str =
        validate s # mail-address validation
        Self.new s

s1 = "invalid mail address"
s2 = "foo@gmail.com"
_ = ValidMailAddressStr.init s1 # panic: invalid mail address
valid = ValidMailAddressStr.init s2
valid: ValidMailAddressStr # assurance that it is in the correct email address format
```

另一個指標是“記名的多相 = 想實現多態”的情況。例如，下面定義的過程接受任何類型為<gtr=“48”/>的對象。但顯然，應用類型對像是錯誤的。因此，我們將參數類型設置為類<gtr=“50”/>。在這種情況下，只有<gtr=“51”/>對象和繼承它的類<gtr=“52”/>對像作為參數。這樣更保守，不用承擔不必要的更多責任。


```erg
Named = {name = Str; ...}
Dog = Class {name = Str; breed = Str}
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}
structural_greet! person: Named =
    print! "Hello, my name is {person::name}."
greet! person: Person =
    print! "Hello, my name is {person::name}."

max = Dog.new {name = "Max", breed = "Labrador"}
john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

structural_greet! max # Hello, my name is Max.
structural_greet! john # Hello, my name is John.
greet! alice # Hello, my name is Alice.
greet! max # TypeError:
```

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_nst_vs_sst.md'>Next</a>
</p>