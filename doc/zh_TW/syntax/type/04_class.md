# Class

Erg 中的類通常可以生成其自身的元素（實例）。下面是一個簡單類的示例。


```erg
Person = Class {.name = Str; .age = Nat}
# .newが定義されなかった場合、自動で`Person.new = Person::__new__`となります
Person.
    new name, age = Self::__new__ {.name = name; .age = age}

john = Person.new "John Smith", 25
print! john # <Person object>
print! classof(john) # Person
```

給出的類型（通常為記錄）稱為要求類型（在本例中為<gtr=“21”/>）。可以在<gtr=“22”/>中生成實例。 <gtr=“23”/>只是一條記錄，但它通過<gtr=“24”/>轉換為<gtr=“25”/>實例。生成此類實例的子例程稱為構造函數。上面的類定義了<gtr=“26”/>方法，以便可以省略字段名等。

請注意，如下所示不換行定義會導致語法錯誤。


```erg
Person.new name, age = ... # SyntaxError: cannot define attributes directly on an object
```

> ：這是最近添加的規範，在以後的文檔中可能不受保護。如果發現就報告。

## 實例屬性、類屬性

在 Python 和其他語言中，很多情況下都是在塊端定義實例屬性，如下所示，這種寫法在 Erg 中是另外一個意思，需要注意。


```python
# Python
class Person:
    name: str
    age: int
```


```erg
# Ergでこの書き方はクラス屬性の宣言を意味する(インスタンス屬性ではない)
Person = Class()
Person.
    name: Str
    age: Int
```


```erg
# 上のPythonコードに対応するErgコード
Person = Class {
    .name = Str
    .age = Nat
}
```

元素屬性（在記錄中定義的屬性）和類型屬性（在類中特別稱為實例屬性/類屬性）是完全不同的。類型屬性是類型本身所具有的屬性。類型的要素在自身中沒有目標屬性時參照類型屬性。要素屬性是要素直接具有的固有屬性。為什麼要做這樣的劃分？如果全部都是要素屬性，則在生成對象時需要復制、初始化所有屬性，這是因為效率低下。另外，這樣分開的話，“這個屬性是共用的”“這個屬性是分開擁有的”等作用就會明確。

用下面的例子來說明。由於這一屬性在所有實例中都是共通的，所以作為類屬性更為自然。但是，由於這一屬性應該是各個實例各自持有的，所以應該是實例屬性。


```erg
Person = Class {name = Str}
Person::
    species = "human"
Person.
    describe() =
        log "species: {species}"
    greet self =
        log "Hello, My name is {self::name}."

Person.describe() # species: human
Person.greet() # TypeError: unbound method Person.greet needs an argument

john = Person.new {name = "John"}
john.describe() # species: human
john.greet() # Hello, My name is John.

alice = Person.new {name = "Alice"}
alice.describe() # species: human
alice.greet() # Hello, My name is Alice.
```

順便一提，如果實例屬性和類型屬性中存在同名、同類型的屬性，就會出現編譯錯誤。這是為了避免混亂。


```erg
C = Class {.i = Int}
C.
    i = 1 # AttributeError: `.i` is already defined in instance fields
```

## Class, Type

請注意，類類型與不同。只有一個類可以從中生成<gtr=“31”/>。可以使用<gtr=“33”/>或<gtr=“34”/>獲取對象所屬的類。與此相對，<gtr=“35”/>有無數個類型。例如，<gtr=“36”/>。但是，最小的類型可以是一個，在這種情況下是<gtr=“37”/>。可以通過<gtr=“38”/>獲取對象的類型。這是一個編譯時函數，顧名思義，它是在編譯時計算的。除了類方法外，對像還可以使用修補程序方法。 Erg 不能添加類方法，但可以使用<gtr=“39”/>進行擴展。

也可以繼承現有的類（對於類）。 <gtr=“40”/>表示繼承。左邊的類型稱為派生類，右邊的<gtr=“41”/>參數類型稱為基類。


```erg
MyStr = Inherit Str
# other: StrとしておけばMyStrでもOK
MyStr.
    `-` self, other: Str = self.replace other, ""

abc = MyStr.new("abc")
# ここの比較はアップキャストが入る
assert abc - "b" == "ac"
```

與 Python 不同，定義的 Erg 類缺省為（不可繼承）。要使其可繼承，必須為類指定<gtr=“44”/>裝飾器。 <gtr=“45”/>是可繼承類之一。


```erg
MyStr = Inherit Str # OK
MyStr2 = Inherit MyStr # NG

@Inheritable
InheritableMyStr = Inherit Str
MyStr3 = Inherit InheritableMyStr # OK
```

和<gtr=“47”/>在實際應用中大致等效。一般使用後者。

類的等價機制不同於類型。類型根據結構確定等價性。


```erg
Person = {.name = Str; .age = Nat}
Human = {.name = Str; .age = Nat}

assert Person == Human
```

類沒有定義等價關係。


```erg
Person = Class {.name = Str; .age = Nat}
Human = Class {.name = Str; .age = Nat}

Person == Human # TypeError: cannot compare classes
```

## 與結構類型的區別

類是一種可以生成自己元素的類型，但這並不是一個嚴格的描述。因為實際上，記錄類型 + 修補程序也可以做到這一點。


```erg
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    new name, age = {.name; .age}

john = Person.new("John Smith", 25)
```

使用類有四個好處。一是檢查構造函數的合法性，二是性能高，三是可以使用記名部分類型 (NST)，四是可以繼承和覆蓋。

我們已經看到記錄類型 + 修補程序也可以定義構造函數（類似），但這當然不是合法的構造函數。因為你可以返回一個自稱但完全不相關的對象。對於類，將靜態檢查是否生成滿足要求的對象。

~

類類型檢查只需查看對象的屬性即可完成。因此，檢查對像是否屬於該類型的速度較快。

~

Erg 在類中提供了 NST。 NST 的優點包括強健性。在編寫大型程序時，對象的結構仍然會偶然匹配。


```erg
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

雖然和的結構完全相同，但允許動物打招呼和人類吠叫顯然是無稽之談。且不說後者，讓前者不適用更安全，因為前者是不可能的。在這種情況下，最好使用類。


```erg
Dog = Class {.name = Str; .age = Nat}
Dog.
    bark = log "Yelp!"
...
Person = Class {.name = Str; .age = Nat}
Person.
    greet self = log "Hello, my name is {self.name}."

john = Person.new {.name = "John Smith"; .age = 20}
john.bark() # TypeError: `Person` object has no method `.bark`
```

另一個特徵是，通過修補程序添加的類型屬性是虛擬的，而不是作為實體保存在要實現的類中。也就是說，和<gtr=“54”/>是與<gtr=“55”/>兼容的類型可以訪問（在編譯時綁定）的對象，而不是在<gtr=“56”/>或<gtr=“57”/>中定義的對象。相反，類屬性由類自己維護。因此，結構相同但不具有繼承關係的類無法訪問。


```erg
C = Class {i = Int}
C.
    foo self = ...
print! dir(C) # ["foo", ...]

T = Patch {i = Int}
T.
    x = 1
    bar self = ...
print! dir(T) # ["bar", "x", ...]
assert T.x == 1
assert {i = 1}.x == 1
print! T.bar # <function bar>
{i = Int}.bar # TypeError: Record({i = Int}) has no method `.bar`
C.bar # TypeError: C has no method `.bar`
print! {i = 1}.bar # <method bar>
print! C.new({i = 1}).bar # <method bar>
```

## 與數據類的區別

類可以是通過請求記錄的常規類，也可以是繼承記錄（<gtr=“59”/>）的數據類。數據類繼承了記錄的功能，可以分解賦值，缺省情況下實現<gtr=“60”/>和<gtr=“61”/>。相反，如果你想定義自己的等價關係和格式顯示，則可以使用常規類。


```erg
C = Class {i = Int}
c = C.new {i = 1}
d = C.new {i = 2}
print! c # <C object>
c == d # TypeError: `==` is not implemented for `C`

D = Inherit {i = Int}
e = D::{i = 1} # e = D.new {i = 1}と同じ
f = D::{i = 2}
print! e # D(i = 1)
assert e != f
```

## Enum Class

提供以幫助定義 Or 類型的類。


```erg
X = Class()
Y = Class()
XorY = Enum X, Y
```

每種類型都可以按和<gtr=“64”/>進行訪問，構造函數可以按<gtr=“65”/>進行檢索。是接收類並返回其構造函數的方法。


```erg
x1 = XorY.new X.new()
x2 = XorY.cons(X)()
assert x1 == x2
```

## 包含關係

類是需求類型的子類型。你可以使用要求類型的方法（包括修補程序方法）。


```erg
T = Trait {.foo = Foo}
C = Class(..., Impl: T)
C.
    foo = foo
    bar x = ...
assert C < T
assert C.foo == foo
assert not T < C
assert T.foo == Foo
```

<p align='center'>
    <a href='./03_trait.md'>Previous</a> | <a href='./05_inheritance.md'>Next</a>
</p>