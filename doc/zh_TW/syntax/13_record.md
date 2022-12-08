# 記錄(Record)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/13_record.md%26commit_hash%3D00350f64a40b12f763a605bc16748d09379ab182)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/13_record.md&commit_hash=00350f64a40b12f763a605bc16748d09379ab182)

記錄是一個集合，它結合了通過鍵訪問的 Dict 和在編譯時檢查其訪問的元組的屬性
如果您了解 JavaScript，請將其視為一種(更增強的)對象字面量表示法

```python
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # 錯誤: john 不可訂閱
```

`.name` 和 `.age` 部分稱為屬性，而 `"John"` 和 `21` 部分稱為屬性值

與 JavaScript 對象字面量的區別在于它們不能作為字符串訪問。也就是說，屬性不僅僅是字符串
這是因為對值的訪問是在編譯時確定的，而且字典和記錄是不同的東西。換句話說，`{"name": "John"}` 是一個字典，`{name = "John"}` 是一個記錄
那么我們應該如何使用字典和記錄呢?
一般來說，我們建議使用記錄。記錄具有在編譯時檢查元素是否存在以及能夠指定 __visibility_ 的優點
指定可見性等同于在 Java 和其他語言中指定公共/私有。有關詳細信息，請參閱 [可見性](./19_visibility.md) 了解詳細信息

```python
a = {x = 1; .y = x + 1}
a.x # 屬性錯誤: x 是私有的
# 提示: 聲明為 `.x`
assert a.y == 2
```

對于熟悉 JavaScript 的人來說，上面的示例可能看起來很奇怪，但簡單地聲明 `x` 會使其無法從外部訪問

您還可以顯式指定屬性的類型

```python
anonymous = {
    .name: Option! Str = !
    .age = 20
}
anonymous.name.set! "John"
```

一個記錄也可以有方法

```python
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

關于記錄有一個值得注意的語法。當記錄的所有屬性值都是類(不是結構類型)時，記錄本身表現為一個類型，其自身的屬性作為必需屬性
這種類型稱為記錄類型。有關詳細信息，請參閱 [記錄] 部分

```python
# 記錄
john = {.name = "John"}
# 記錄 type
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am \{n.name}"
john # "你好，我是約翰 print！

Named.name # Str
```

## 解構記錄

記錄可以按如下方式解構

```python
record = {x = 1; y = 2}
{x = a; y = b} = record
assert a == 1
assert b == 2

point = {x = 2; y = 3; z = 4}
match point:
    {x = 0; y = 0; z = 0} -> "origin"
    {x = _; y = 0; z = 0} -> "on the x axis"
    {x = 0; ...} -> "x = 0"
    {x = x; y = y; z = z} -> "(\{x}, \{y}, \{z})"
```

當存在與屬性同名的變量時，`x = ...`也可以縮寫為`x`，例如`x = x`或`x = .x`到`x`，和` .x = .x` 或 `.x = x` 到 `.x`
但是，當只有一個屬性時，必須在其后加上`;`以與集合區分開來

```python
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

此語法可用于解構記錄并將其分配給變量

```python
# 一樣 `{x = x; y = y} = xy`
{x; y} = xy
assert x == 1
assert y == 2
# 一樣 `{.a = a; .b = b} = ab`
{a; b} = ab
assert a == 1
assert b == 2
```

## 空記錄

空記錄由`{=}`表示。空記錄也是它自己的類，如 Unit

```python
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

空記錄不同于空 Dict `{:}` 或空集 `{}`。特別要注意的是，它與 `{}` 的含義相反(在 Python 中，`{}` 是一個空字典，而在 Erg 中它是 Erg 中的 `!{:}`)
作為枚舉類型，`{}` 是一個空類型，其元素中不包含任何內容。`Never` 類型是這種類型的一個分類
相反，記錄類 `{=}` 沒有必需的實例屬性，因此所有對象都是它的元素。`Object` 是 this 的別名
一個`Object`(`Object`的一個補丁)是`的一個元素。__sizeof__` 和其他非常基本的提供方法

```python
AnyPatch = Patch Structural {=}
    . __sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

請注意，沒有其他類型或類在結構上與 `{}`、`Never` 類型等效，如果用戶在右側使用 `{}`、`Class {}` 定義類型，則會出錯
這意味著，例如，`1..10 或 -10。-1`，但 `1..10 和 -10... -1`。例如，當它應該是 1..10 或 -10...-1 時是 `-1`
此外，如果您定義的類型(例如 `Int 和 Str`)會導致組合 `Object`，則會警告您只需將其設置為 `Object`

## 即時封鎖

Erg 有另一種語法 Instant 塊，它只返回最后評估的值。不能保留屬性

```python
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # 語法錯誤: 無法在實體塊中定義屬性
```

## 數據類

如果您嘗試自己實現方法，則必須直接在實例中定義裸記錄(由記錄文字生成的記錄)
這是低效的，并且隨著屬性數量的增加，錯誤消息等變得難以查看和使用

```python
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is \{self::name} and I am \{self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
john + 1
# 類型錯誤: {name = Str; 沒有實現 + 年齡=詮釋； 。迎接！ =參考(自我)。() => 無； inc_age！ =參考！ () => 無}, 整數
```

因此，在這種情況下，您可以繼承一個記錄類。這樣的類稱為數據類
這在 [class](./type/04_class.md) 中有描述

```python
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is \{self::name} and I am \{self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
john + 1
# 類型錯誤: Person、Int 沒有實現 +
```

<p align='center'>
    <a href='./12_tuple.md'>上一頁</a> | <a href='./14_set.md'>下一頁</a>
</p>
