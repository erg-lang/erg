# 記錄

記錄是一個集合，它具有通過鍵訪問的 Dict 和在編譯時檢查訪問的元組的性質。如果你使用過 JavaScript，請將其視為對象文字符號（更高級）。


```erg
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # Error: john is not subscribable
```

和<gtr=“15”/>部分稱為屬性，<gtr=“16”/>和<gtr=“17”/>部分稱為屬性值。

它與 JavaScript 對象文字的區別在於不能以字符串形式訪問。也就是說，屬性不僅僅是字符串。這可能是因為它在編譯時決定對值的訪問，也可能是因為字典和記錄是不同的。也就是說，是 Dict，<gtr=“19”/>是記錄。那麼，詞典和記錄該如何區分使用呢？通常建議使用記錄。記錄具有以下優點：編譯時檢查元素是否存在，並且可以指定<gtr=“21”/>。可見性規範相當於 public/private 規範，例如在 Java 語言中。有關詳細信息，請參見<gtr=“20”/>。


```erg
a = {x = 1; .y = x + 1}
a.x # AttributeError: x is private
# Hint: declare as `.x`
assert a.y == 2
```

對於熟悉 JavaScript 的人來說，上面的例子可能很奇怪，但如果簡單地聲明，則外部無法訪問，如果加上<gtr=“23”/>，則可以通過<gtr=“24”/>訪問。

還可以顯式指定屬性的類型。


```erg
anonymous = {
    .name: Option! Str = !None
    .age = 20
}
anonymous.name.set! "John"
```

記錄也可以有方法。


```erg
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

關於記錄有一個值得注意的語法。當記錄的所有屬性值都是類（結構類型不允許）時，記錄本身將其屬性視為請求屬性。這種類型稱為記錄類型。有關詳細信息，請參閱記錄部分。


```erg
# レコード
john = {.name = "John"}
# レコード型
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am {n.name}"
greet! john # "Hello, I am John"

print! Named.name # Str
```

## 分解記錄

可以按如下方式分解記錄。


```erg
record = {x = 1; y = 2}
{x = a; y = b} = record
assert a == 1
assert b == 2

point = {x = 2; y = 3; z = 4}
match point:
    {x = 0; y = 0; z = 0} -> "origin"
    {x = _; y = 0; z = 0} -> "on the x axis"
    {x = 0; ...} -> "x = 0"
    {x = x; y = y; z = z} -> "({x}, {y}, {z})"
```

此外，如果記錄具有與屬性同名的變量，則可以將或<gtr=“26”/>省略為<gtr=“27”/>，將<gtr=“28”/>或<gtr=“29”/>省略為<gtr=“30”/>。但是，如果只有一個屬性，則必須使用<gtr=“31”/>將其與集合區分開來。


```erg
x = 1
y = 2
xy = {x; y}
a = 1
b = 2
ab = {.a; .b}
assert ab.a == 1
assert ab.b == 2

record = {x;}
tuple = {x}
assert tuple.1 == 1
```

此語法可用於分解記錄並將其賦給變量。


```erg
# same as `{x = x; y = y} = xy`
{x; y} = xy
assert x == 1
assert y == 2
# same as `{.a = a; .b = b} = ab`
{a; b} = ab
assert a == 1
assert b == 2
```

## 空記錄

空記錄由表示。與 Unit 一樣，空記錄也是其類本身。


```erg
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

空記錄不同於空 Dict或空集<gtr=“34”/>。尤其要注意它與<gtr=“35”/>的含義正好相反（在 Python 中，<gtr=“36”/>是一個空字典，而在 Erg 中，它是<gtr=“37”/>）。作為枚舉類型，<gtr=“38”/>是空類型，不包含任何元素。類型是對其進行的類化。相反，記錄類中的<gtr=“40”/>沒有請求實例屬性，因此所有對像都是它的元素。是此別名。 <gtr=“42”/>（修補程序）具有非常基本的提供方法，如<gtr=“43”/>。


```erg
AnyPatch = Patch Structural {=}
    .__sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

請注意，沒有其他類型和類在結構上與，<gtr=“45”/>類型等效，如果用戶定義類型時在右邊指定<gtr=“46”/>，<gtr=“47”/>，則會出錯。這可以防止將<gtr=“48”/>轉換為<gtr=“49”/>的錯誤。此外，如果定義組合結果為<gtr=“50”/>的類型（例如<gtr=“51”/>），則會發出警告，將其簡單地定義為<gtr=“52”/>。

## 即時塊

Erg 還有一個語法叫即時塊，它只是返回最後評估的值。不能保留屬性。


```erg
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # SyntaxError: cannot define an attribute in an entity block
```

## 數據類

如果嘗試單獨實現方法，則必須直接在實例中定義原始記錄（由記錄文本生成的記錄）。這效率很低，而且隨著屬性數量的增加，錯誤顯示等很難看到，也很難使用。


```erg
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
john + 1
# TypeError: + is not implemented for {name = Str; age = Int; .greet! = Ref(Self).() => None; inc_age! = Ref!(Self).() => None}, Int
```

因此，在這種情況下，我們將繼承記錄類。此類類稱為數據類。我們將在部分詳細討論這一點。


```erg
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
john + 1
# TypeError: + is not implemented for Person, Int
```

<p align='center'>
    <a href='./12_dict.md'>Previous</a> | <a href='./14_set.md'>Next</a>
</p>