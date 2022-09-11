# 術語統一

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/unify_terms.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/unify_terms.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## 可訪問性，可見性

使用可見性。

## 類型綁定，類型約束

給定量化和細化類型的謂詞表達式列表。使用類型邊界。

## 子程序、例程、子程序

使用子程序。

## 引用透明/不透明，有/沒有副作用

使用有/無副作用。

## 標識符、代數、變量、名稱、符號

就其本義而言，

* 符號：在源代碼中實心編寫的字符(符號、控制字符等除外)，不是字符串對象(未包含在“”中)。符號在 Ruby、Lisp 等中作為原始類型存在，但在 Erg 中它們不被視為對象。
* 標識符：(并且可以)引用某個對象的符號，而不是保留字。例如，在 Python 中 class 和 def 不能用作標識符。由于 Erg 沒有保留字，所以除了某些符號外，所有符號都可以用作標識符。
* 名稱：與標識符的含義幾乎相同。它有時與 Erg 中的代數同義使用。
* 代數名稱：相當于Erg中的標識符。在 C 中，函數名稱是標識符，而不是代數名稱。 “代數”指的是語言特性本身，它允許您使用 `=`(變量賦值運算符)或 `=`(常量賦值運算符)來分配對象。

```python
代數名稱<：(名稱==標識符)??<：符號
變量 + 常數 == 代數
```

然而，應該稱為“代數”的東西，往往被稱為“變量”。這就是數學術語的效果。
值內容可以改變的變量是可變變量，值內容不改變的變量是不可變變量。
請注意，常量始終是不可變的。

Erg 中不使用代數名稱和名稱，使用統一標識符。
但是，一般來說，具有 `v = 1` 的 `v` 稱為“變量 v”，而具有 `C = 1` 的 `C` 稱為“常量 C”。 .

## 屬性、字段、屬性

使用屬性。順便說一句，記錄是一個函數，它可以定義一個沒有類的具有元素屬性的對象。

## 應用程序，調用

為子例程對象提供參數并獲得結果。
使用呼叫。這是因為Application有“應用軟件”的用法。

## 數組列表

使用數組。 Erg 數組(通常)在內存中是連續的。
List 是指所謂的鏈表，或者說列表作為 Python 數據類型。

## lambda 函數、lambda 表達式、匿名函數

與匿名函數統一。在英文中，可以使用 Lambda 來縮短字符數，但正式名稱是 Anonymous function。