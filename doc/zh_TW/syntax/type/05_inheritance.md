# 繼承

繼承允許您定義一個新類，為現有類添加功能或專業化。
繼承類似于包含在特征中。 繼承的類成為原始類的子類型。

```python
NewInt = Inherit Int
NewInt.
    plus1 self = self + 1

assert NewInt.new(1).plus1() == 2
assert NewInt.new(1) + NewInt.new(1) == 2
```

如果你希望新定義的類是可繼承的，你必須給它一個 `Inheritable` 裝飾器。

您可以指定一個可選參數 `additional` 以允許該類具有其他實例屬性，但前提是該類是一個值類。 但是，如果類是值類，則不能添加實例屬性。

```python
@Inheritable
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}

john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

MailAddress = Inherit Str, additional: {owner = Str} # 類型錯誤：實例變量不能添加到值類中
```

Erg 的特殊設計不允許繼承“Never”類型。 Erg 的特殊設計不允許繼承 `Never` 類型，因為 `Never` 是一個永遠無法實例化的獨特類。

## 枚舉類的繼承

[Or 類型](./13_algebraic.md) 也可以被繼承。 在這種情況下，您可以通過指定可選參數 `Excluding` 來刪除任何選項(可以使用 `or` 進行多項選擇)。
不能添加其他選項。 添加選項的類不是原始類的子類型。

```python
Number = Class Int or Float or Complex
Number.abs(self): Float =
    match self:
        i: Int -> i.abs().into Float
        f: Float -> f.abs()
        c: Complex -> c.abs().into Float

# c: 復雜不能出現在匹配選項中
RealNumber = Inherit Number, Excluding: Complex
```

同樣，也可以指定[細化類型](./12_refinement.md)。

```python
Months = Class 0..12
MonthsNot31Days = Inherit Months, Excluding: {1, 3, 5, 7, 8, 10, 12}

StrMoreThan3 = Class StrWithLen N | N >= 3
StrMoreThan4 = Inherit StrMoreThan3, Excluding: StrWithLen N | N == 3
```

## 覆蓋

該類與補丁相同，可以將新方法添加到原始類型，但可以進一步“覆蓋”該類。
這種覆蓋稱為覆蓋。要覆蓋，必須滿足三個條件。
首先，覆蓋必須有一個 `Override` 裝飾器，因為默認情況下它會導致錯誤。
另外，覆蓋不能改變方法的類型。它必須是原始類型的子類型。
如果你重寫了一個被另一個方法引用的方法，你也必須重寫所有被引用的方法。

為什么這個條件是必要的？這是因為重寫不僅會改變一種方法的行為，而且可能會影響另一種方法的行為。

讓我們從第一個條件開始。此條件是為了防止“意外覆蓋”。
換句話說，必須使用 `Override` 裝飾器來防止派生類中新定義的方法的名稱與基類的名稱沖突。

接下來，考慮第二個條件。這是為了類型一致性。由于派生類是基類的子類型，因此它的行為也必須與基類的行為兼容。

最后，考慮第三個條件。這種情況是 Erg 獨有的，在其他面向對象語言中并不常見，同樣是為了安全。讓我們看看如果不是這種情況會出現什么問題。

```python
# 反面示例
@Inheritable
Base! = Class {x = Int!}
Base!
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!
    @Override
    g! ref! self = self.f!() # 無限遞歸警告：此代碼陷入無限循環 
    # 覆蓋錯誤：方法 `.g` 被 `.f` 引用但未被覆蓋
```

在繼承類 `Inherited!` 中，`.g!` 方法被重寫以將處理轉移到 `.f!`。 但是，基類中的 `.f!` 方法會將其處理轉移到 `.g!`，從而導致無限循環。 `.f` 是 `Base!` 類中的一個沒有問題的方法，但它被覆蓋以一種意想不到的方式使用，并且被破壞了。

Erg 已將此規則構建到規范中。

```python
# OK.
@Inheritable
Base! = Class {x = Int!}
Base!
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!
    @Override
    f! ref! self =
        print! self::x
        self::x.update! x -> x + 1
    @Override
    g! ref! self = self.f!()
```

然而，這個規范并沒有完全解決覆蓋問題。 然而，這個規范并沒有完全解決覆蓋問題，因為編譯器無法檢測覆蓋是否解決了問題。
創建派生類的程序員有責任糾正覆蓋的影響。 只要有可能，嘗試定義一個別名方法。

### 替換特征(或看起來像什么)

盡管無法在繼承時替換特征，但有一些示例似乎可以這樣做。

例如，`Int`，`Real` 的子類型(實現了 `Add()`)，似乎重新實現了 `Add()`。

```python
Int = Class ... , Impl := Add() and ...
```

但實際上 `Real` 中的 `Add()` 代表 `Add(Real, Real)`，而在 `Int` 中它只是被 `Add(Int, Int)` 覆蓋。
它們是兩個不同的特征(`Add` 是一個 [covariate](./advanced/variance.md)，所以`Add(Real, Real) :> Add(Int, Int)`)。

## 多重繼承

Erg 不允許普通類之間的交集、差異和互補。
```python
Int and Str # 類型錯誤：無法合并類
```

該規則防止從多個類繼承，即多重繼承。

```python
IntAndStr = Inherit Int and Str # 語法錯誤：不允許類的多重繼承
```

但是，可以使用多個繼承的 Python 類。

## 多層(多級)繼承

Erg 繼承也禁止多層繼承。 也就是說，您不能定義從另一個類繼承的類。
從“Object”繼承的可繼承類可能會異常繼承。

同樣在這種情況下，可以使用 Python 的多層繼承類。

## 重寫繼承的屬性

Erg 不允許重寫從基類繼承的屬性。 這有兩個含義。

第一個是對繼承的源類屬性的更新操作。 例如，它不能重新分配，也不能通過 `.update!` 方法更新。

覆蓋與重寫不同，因為它是一種用更專業的方法覆蓋的操作。 覆蓋也必須替換為兼容的類型。

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!
    var = !1
    inc_pub! ref! self = self.pub.update! p -> p + 1

Inherited! = Inherit Base!
Inherited!
    var.update! v -> v + 1
    # 類型錯誤：不能更新基類變量
    @Override
    inc_pub! ref! self = self.pub + 1
    # 覆蓋錯誤：`.inc_pub!` 必須是 `Self! 的子類型！ () => ()`
```

第二個是對繼承源的(變量)實例屬性的更新操作。 這也是被禁止的。 基類的實例屬性只能從基類提供的方法中更新。
無論屬性的可見性如何，都無法直接更新。 但是，它們可以被讀取。

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!
    inc_pub! ref! self = self.pub.update! p -> p + 1
    inc_pri! ref! self = self::pri.update! p -> p + 1

self = self.pub.update!
Inherited!
    # OK
    add2_pub! ref! self =
        self.inc_pub!()
        self.inc_pub!()
    # NG, `Child` 不能觸摸 `self.pub` 和 `self::pri`。
    add2_pub! ref! self =
        self.pub.update! p -> p + 2
```

畢竟 Erg 繼承只能添加新的屬性和覆蓋基類的方法。

## 使用繼承

雖然繼承在正確使用時是一項強大的功能，但它也有一個缺點，即它往往會使類依賴關系復雜化，尤其是在使用多層或多層繼承時。復雜的依賴關系會降低代碼的可維護性。
Erg 禁止多重和多層繼承的原因是為了降低這種風險，并且引入了類補丁功能以降低依賴關系的復雜性，同時保留繼承的“添加功能”方面。

那么，反過來說，應該在哪里使用繼承呢？一個指標是何時需要“基類的語義子類型”。
Erg 允許類型系統自動進行部分子類型確定(例如，Nat，其中 Int 大于或等于 0)。
但是，例如，僅依靠 Erg 的類型系統很難創建“表示有效電子郵件地址的字符串類型”。您可能應該對普通字符串執行驗證。然后，我們想為已通過驗證的字符串對象添加某種“保證”。這相當于向下轉換為繼承的類。將 `Str object` 向下轉換為 `ValidMailAddressStr` 與驗證字符串是否采用正確的電子郵件地址格式是一一對應的。

```python
ValidMailAddressStr = Inherit Str
ValidMailAddressStr.
    init s: Str =
        validate s # 郵件地址驗證
        Self.new s

s1 = "invalid mail address"
s2 = "foo@gmail.com"
_ = ValidMailAddressStr.init s1 # 恐慌：無效的郵件地址
valid = ValidMailAddressStr.init s2
valid: ValidMailAddressStr # 確保電子郵件地址格式正確
```

另一個指標是您何時想要實現名義多態性。
例如，下面定義的 `greet!` 過程將接受任何類型為 `Named` 的對象。
但顯然應用 `Dog` 類型的對象是錯誤的。 所以我們將使用 `Person` 類作為參數類型。
這樣，只有 `Person` 對象、從它們繼承的類和 `Student` 對象將被接受為參數。
這是比較保守的，避免不必要地承擔過多的責任。

```python
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

structural_greet! max # 你好，我是馬克斯
structural_greet! john # 你好，我是約翰
greet! alice # 你好，我是愛麗絲
greet! max # 類型錯誤：
```

<p align='center'>
    <a href='./04_class.md'>上一頁</a> | <a href='./06_nst_vs_sst.md'>下一頁</a>
</p>
