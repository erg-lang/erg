# 對象系統

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/26_object_system.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/26_object_system.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

可以分配給變量的所有數據。`Object` 類的屬性如下

* `.__repr__`: 返回對象的(非豐富)字符串表示
* `.__sizeof__`: 返回對象的大小(包括堆分配)
* `.__dir__`: 返回對象屬性列表
* `.__hash__`: 返回對象的哈希值
* `.__getattribute__`: 獲取并返回對象的屬性
* `.clone`: 創建并返回一個對象的克隆(在內存中有一個獨立的實體)
* `.copy`: 返回對象的副本(指向內存中的同一事物)

## 記錄

由記錄文字(`{attr = value; ...}`)生成的對象
這個對象有基本的方法，比如`.clone`和`.__sizeof__`

```python
obj = {.x = 1}
assert obj.x == 1

obj2 = {...x; .y = 2}
assert obj2.x == 1 and obj2.y == 2
```

## 屬性

與對象關聯的對象。特別是，將 self (`self`) 作為其隱式第一個參數的子例程屬性稱為方法

```python
# 請注意，private_attr 中沒有`.`
record = {.public_attr = j; private_attr = 2; .method = self -> self.i + 1}
record. public_attr == 2
record.private_attr # AttributeError: private_attr 是私有的
assert record.method() == 3
```

## 元素

屬于特定類型的對象(例如，"1"是"Int"類型的元素)。所有對象至少是`{=}`類型的元素
類的元素有時稱為實例

## 子程序

表示作為函數或過程(包括方法)實例的對象。代表子程序的類是"子程序"
實現 `.__call__` 的對象通常稱為 `Callable`

## 可調用

一個實現`.__call__`的對象。它也是 `Subroutine` 的父類

## 類型

定義需求屬性并使對象通用化的對象
主要有兩種類型: 多態類型和單態類型。典型的單態類型有`Int`、`Str`等，多態類型有`Option Int`、`[Int; 3]`等
此外，定義改變對象狀態的方法的類型稱為 Mutable 類型，需要在變量屬性中添加 `!`(例如動態數組: `[T; !_]`)

## 班級

具有 `.__new__`、`.__init__` 方法等的類型。實現基于類的面向對象

## 功能

對外部變量(不包括靜態變量)有讀權限但對外部變量沒有讀/寫權限的子程序。換句話說，它沒有外部副作用
Erg 函數的定義與 Python 的不同，因為它們不允許副作用

## 程序

它對外部變量具有讀取和"自我"權限，對靜態變量具有讀/寫權限，并允許使用所有子例程。它可能有外部副作用

## 方法

隱式將"self"作為第一個參數的子例程。它與簡單的函數/過程是不同的類型

## 實體

不是子例程和類型的對象
單態實體(`1`、`"a"` 等)也稱為值對象，多態實體(`[1, 2, 3], {"a": 1}`)也稱為容器對象

<p align='center'>
    <a href='./25_module.md'>上一頁</a> | <a href='./27_pattern_matching.md'>下一頁</a>
</p>