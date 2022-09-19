# Class

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/04_class.md%26commit_hash%3D157f51ae0e8cf3ceb45632b537ebe3560a5500b7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/04_class.md&commit_hash=157f51ae0e8cf3ceb45632b537ebe3560a5500b7)

Erg 中的類大致是一種可以創建自己的元素(實例)的類型。
這是一個簡單類的示例。

```python
Person = Class {.name = Str; .age = Nat}
# 如果 `.new` 沒有定義，那麼 Erg 將創建 `Person.new = Person::__new__`
Person.
    new name, age = Self::__new__ {.name = name; .age = age}

john = Person.new "John Smith", 25
print! john # <Person object>
print! classof(john) # Person
```

賦予"Class"的類型(通常是記錄類型)稱為需求類型(在本例中為"{.name = Str; .age = Nat}")。
可以使用 `<Class name>::__new__ {<attribute name> = <value>; 創建實例 ...}` 可以創建。
`{.name = "約翰·史密斯"; .age = 25}` 只是一條記錄，但它通過傳遞 `Person.new` 轉換為 `Person` 實例。
創建此類實例的子例程稱為構造函數。
在上面的類中，`.new` 方法被定義為可以省略字段名等。

請注意，以下不帶換行符的定義將導致語法錯誤。

```python
Person.new name, age = ... # 語法錯誤：不能直接在對像上定義屬性
```

> __Warning__：這是最近添加的規範，後續文檔中可能不會遵循。如果你發現它，請報告它。

## 實例和類屬性

在 Python 和其他語言中，實例屬性通常在塊側定義如下，但請注意，這樣的寫法在 Erg 中具有不同的含義。

```python
# Python
class Person:
    name: str
    age: int
```

```python
# 在Erg中，這個符號意味著類屬性的聲明(不是實例屬性)
Person = Class()
Person.
    name: Str
    age: Int
```

```python
# 以上 Python 代碼的 Erg 代碼
Person = Class {
    .name = Str
    .age = Nat
}
```

元素屬性(在記錄中定義的屬性)和類型屬性(也稱為實例/類屬性，尤其是在類的情況下)是完全不同的東西。類型屬性是類型本身的屬性。當一個類型的元素本身沒有所需的屬性時，它指的是一個類型屬性。元素屬性是元素直接擁有的唯一屬性。
為什麼要進行這種區分?如果所有屬性都是元素屬性，那麼在創建對象時復制和初始化所有屬性將是低效的。
此外，以這種方式劃分屬性明確了諸如"該屬性是共享的"和"該屬性是分開持有的"之類的角色。

下面的例子說明了這一點。 `species` 屬性對所有實例都是通用的，因此將其用作類屬性更自然。但是，屬性 `name` 應該是實例屬性，因為每個實例都應該單獨擁有它。

```python
Person = Class {name = Str}
Person::
    species = "human"
Person.
    describe() =
        log "species: {species}"
    greet self =
        log "Hello, My name is {self::name}."

Person.describe() # 類型：Person
Person.greet() # 類型錯誤: 未綁定的方法 Person.greet 需要一個參數

john = Person.new {name = "John"}
john.describe() # 類型: human
john.greet() # 你好，我是約翰

alice = Person.new {name = "Alice"}
alice.describe() # 類型: human
alice.greet() # 你好，我是愛麗絲
```

順便說一下，如果實例屬性和類型屬性具有相同的名稱和相同的類型，則會發生編譯錯誤。這是為了避免混淆。

```python
C = Class {.i = Int}
C.i = 1 # 屬性錯誤：`.i` 已在實例字段中定義
```

## 類(Class), 類型(Type)

請注意，`1` 的類和類型是不同的。
只有一個類 `Int` 是 `1` 的生成器。可以通過`classof(obj)`或`obj.__class__`獲取對象所屬的類。
相比之下，`1`有無數種。例如，`{1}, {0, 1}, 0..12, Nat, Int, Num`。
但是，可以將最小類型定義為單一類型，在本例中為"{1}"。可以通過`Typeof(obj)`獲取對象所屬的類型。這是一個編譯時函數。
對象可以使用補丁方法以及類方法。
Erg 不允許您添加類方法，但您可以使用 [patch](./07_patch.md) 來擴展類。

您還可以從現有類([Inheritable](./../27_decorator.md/#inheritable) 類)繼承。
您可以使用 `Inherit` 創建一個繼承類。左側的類型稱為派生類，右側的"繼承"的參數類型稱為基類(繼承類)。

```python
MyStr = Inherit Str
# other: 如果你設置 ``other: Str''，你可以使用 MyStr。
MyStr.
    `-` self, other: Str = self.replace other, ""

abc = MyStr.new("abc")
# 這裡的比較是向上的
assert abc - "b" == "ac"
```

與 Python 不同，默認情況下，定義的 Erg 類是 `final`(不可繼承的)。
要使類可繼承，必須將 `Inheritable` 裝飾器附加到該類。
Str` 是可繼承的類之一。

```python
MyStr = Inherit Str # OK
MyStr2 = Inherit MyStr # NG

@Inheritable
InheritableMyStr = Inherit Str
MyStr3 = Inherit InheritableMyStr # OK
```

`Inherit Object` 和 `Class()` 在實踐中幾乎是等價的。一般使用後者。

類具有與類型不同的等價檢查機制。
類型基於其結構進行等效性測試。

```python
Person = {.name = Str; .age = Nat}
Human = {.name = Str; .age = Nat}

assert Person == Human
```

class has no equivalence relation defined.

```python
Person = Class {.name = Str; .age = Nat}
Human = Class {.name = Str; .age = Nat}

Person == Human # 類型錯誤：無法比較類
```

## 與結構類型的區別

我們說過類是一種可以生成自己的元素的類型，但這並不是嚴格的描述。事實上，一個記錄類型+補丁可以做同樣的事情。

```python
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    new name, age = {.name; .age}

john = Person.new("John Smith", 25)
```

使用類有四個優點。
第一個是構造函數經過有效性檢查，第二個是它的性能更高，第三個是您可以使用符號子類型(NST)，第四個是您可以繼承和覆蓋。

我們之前看到記錄類型 + 補丁也可以定義一個構造函數(某種意義上)，但這當然不是一個合法的構造函數。這當然不是一個合法的構造函數，因為它可以返回一個完全不相關的對象，即使它調用自己`.new`。在類的情況下，`.new` 被靜態檢查以查看它是否生成滿足要求的對象。

~

類的類型檢查只是檢查對象的`。 __class__` 對象的屬性。因此可以快速檢查一個對像是否屬於一個類型。

~

Erg 在課堂上啟用 NST； NST 的優點包括健壯性。
在編寫大型程序時，經常會出現對象的結構巧合匹配的情況。

```python
Dog = {.name = Str; .age = Nat}
DogImpl = Patch Dog
DogImpl.
    bark = log "Yelp!"
...
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    greet self = log "Hello, my name is {self.name}."

john = {.name = "John Smith"; .age = 20}
john.bark() # "Yelp!"
```

`Dog` 和 `Person` 的結構完全一樣，但讓動物打招呼，讓人類吠叫顯然是無稽之談。
前者是不可能的，所以讓它不適用更安全。在這種情況下，最好使用類。

```python
Dog = Class {.name = Str; .age = Nat}
Dog.bark = log "Yelp!"
...
Person = Class {.name = Str; .age = Nat}
Person.greet self = log "Hello, my name is {self.name}."

john = Person.new {.name = "John Smith"; .age = 20}
john.bark() # 類型錯誤: `Person` 對像沒有方法 `.bark`。
```

另一個特點是補丁添加的類型屬性是虛擬的，實現類不作為實體保存。
也就是說，`T.x`、`T.bar` 是可以通過與 `{i = Int}` 兼容的類型訪問(編譯時綁定)的對象，並且未在 `{i = Int}` 或 ` C`。
相反，類屬性由類本身持有。因此，它們不能被不處於繼承關係的類訪問，即使它們具有相同的結構。

```python
C = Class {i = Int}
C.
    foo self = ...
print! dir(C) # ["foo", ...].

T = Patch {i = Int}
T.
    x = 1
    bar self = ...
print! dir(T) # ["bar", "x", ...].
assert T.x == 1
assert {i = 1}.x == 1
print! T.bar # <函數 bar>
{i = Int}.bar # 類型錯誤：Record({i = Int}) 沒有方法 `.bar`。
C.bar # 類型錯誤：C 沒有方法 `.bar` 打印！
print! {i = 1}.bar # <方法 bar>
C.new({i = 1}).bar # <方法 bar>
```

## 與數據類的區別

有兩種類型的類：常規類，通過`Class`成為記錄類，以及從記錄類繼承(`Inherit`)的數據類。
數據類繼承了記錄類的功能，具有分解賦值、默認實現的`==`和`hash`等特性。另一方面，數據類有自己的等價關係和格式展示。
另一方面，如果要定義自己的等價關係或格式顯示，則應使用普通類。

```python
C = Class {i = Int}
c = C.new {i = 1}
d = C.new {i = 2}
print! c # <C object>
c == d # 類型錯誤：`==` 沒有為 `C` 實現

D = Inherit {i = Int}
e = D::{i = 1} # 與`e = D.new {i = 1}`相同
f = D::{i = 2}
print! e # D(i=1)
assert e ! = f
```

## 枚舉類

為了便於定義"Or"類型的類，提供了一個"Enum"。

```python
X = Class()
Y = Class()
XorY = Enum X, Y
```

每種類型都可以通過`XorY.X`、`XorY.Y`來訪問，構造函數可以通過`X.new |> XorY.new`獲得。

```python
x1 = XorY.new X.new()
x2 = (X.new |> XorY.new)()
x3 = (Y.new |> XorY.new)()
assert x1 == x2
assert x1 != x3
```

## 類關係

類是需求類型的子類型。類中可以使用需求類型的方法(包括補丁方法)。

```python
T = Trait {.foo = Foo}
C = Class(... , impl: T)
C.
    foo = foo
    bar x = ...
assert C < T
assert C.foo == foo
assert not T < C
assert T.foo == Foo
```

<p align='center'>
    <a href='./03_trait.md'>上一頁</a> | <a href='./05_inheritance.md'>下一頁</a>
</p>
