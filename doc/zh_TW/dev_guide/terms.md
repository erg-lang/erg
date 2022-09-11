# 詞匯表

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/terms.md%26commit_hash%3D275c35f66b250942fda1ab0cee173ea016e9fd67)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/terms.md&commit_hash=275c35f66b250942fda1ab0cee173ea016e9fd67)

## 符號

### &excl;

添加到標識符末尾的標記以指示它是過程，變量類型或變異運算符。

### [&#35;](../syntax/00_basic.md/#comment)

### $

### %

### &

### &prime; (single quote)

### &lpar;&rpar;

### &ast;

### &plus;

### &comma;

### &minus;

### ->

### &period;

### /

### &colon;

### &colon;&colon;

### &semi;

### &lt;

### &lt;&colon;

### &lt;&lt;

### &lt;=

### =

### ==

### =>

### &gt;

### &gt;&gt;

### &gt;=

### ?

### @

### []

### \

### ^

### ^^

### _

### ``

### {}

### {:}

### {=}

### |

### ||

### ~

## A

### [algebraic&nbsp;type]

### [And]

### [and]

### [assert]

### [attribute]

## B

### [Base]

### [Bool]

## C

### [Class]

## D

### Deprecated

### [distinct]

## E

### [enum&nbsp;type]

### [Eq]

### [Erg]

## F

### [for]

## G

## H

## I

### [if]

### [import]

### [in]

### [Int]

## J

## K

## L

### let-polymorphism -> [rank 1 polymorphism]

### [log]

## M

### [match]

## N

### [Nat]

### Never

### None

### [Not]

### [not]

## O

### [Option]

### [Or]

### [or]

### [Ord]

## P

### panic

### [print!](../syntax/../API/procs.md#print)

### [Python]

## Q

## R

### ref

### ref&excl;

### [Result]

### [rootobj]

## S

### self

### [Self](../syntax/type/special.md)

### [side-effect](../syntax/07_side_effect.md)

### [Str]

## T

### Traits

### [True]

### [Type]

### [type]

## U

## V

## W

### [while!]

## X

## Y

## Z

## A line

### [Assertion]

檢查(通常在運行時)代碼中的條件是否為真。 這是使用 `assert` 函數等來完成的。

```python
sum = !0
for! 0..10, i =>
    sum.add!i

assert sum == 55
```

### 值對象

在 Erg 中，相當于基礎對象。 它可以在編譯時進行評估，并且具有簡單的比較方法。

### [附件補丁](../syntax/29_decorator.md#attach)

為特征提供標準實現的補丁。

### Ad hoc 多態性 -> [無重載](../syntax/type/overloading.md)

具有所謂重載的多態性。

### 屬性-> [屬性]

`x.y` 標識符中的 `y` 部分。

### Arity

運算符需要多少個操作數。

### [依賴類型](../syntax/type/dependent_type.md)

參數是值的類型(習慣上說，不是類型)。

### 不可變 -> [不可變]

表示目標不會改變。
其他語言中的變量也是不可變/可變的，但是在 Erg 中所有的變量都是不可變的。

### 參數 -> [參數]

### 實例

由類創建的對象。 類類型的元素。

### [即時封鎖](../syntax/00_basic.md#expression separator)

```python
x =
    y = f(a)
    z = g(b,c)
    y+z
```

### 指數

形式為“x[i]”，或其“i”部分。我們稱 `x` 為 Indexable 對象。

### [縮進](../syntax/00_basic.md#indent)

通過向空格移動將文本向右對齊。縮進。
Ergs 通過縮進表示塊。這稱為越位規則。

### 別名

別名。

### 錯誤

規范中定義的異常情況。

* [錯誤處理]

### [運算符](../syntax/06_operator.md)

將操作應用于其操作數的對象。或表示該對象的符號。

* [運算符綁定強度]

### 覆蓋

在子類中覆蓋超類方法。
在 Erg 中，您必須在覆蓋時添加 `Override` 裝飾器。

### [不重載](../syntax/type/overloading.md)

### 越位規則-> [縮進](../syntax/00_basic.md#indent)

### [目的]

* 面向對象

### 操作數 -> [操作數](../syntax/06_operator.md)

### 運算符 -> [運算符](../syntax/06_operator.md)

##嘉線

### [種類](../syntax/type/advanced/kind.md)

所謂類型的類型。

### [可見性]

標識符是否可以被外部引用(超出范圍，或在另一個模塊或包中)的屬性。

### [類型]

對術語進行分組的對象。

* [類型規格]
* [類型擦除](../syntax/type/advanced/erasure.md)
* [類型推斷]
* [類型注釋](../syntax/type/conv_type.md)
* [類型參數]
* [類型添加](../syntax/type/advanced/erasure.md)
* [類型變量](../syntax/type/type_variable.md)
* [類型約束]

### 監視

### 封裝

隱藏實現細節。

### [多變的]

不能是一成不變的。

* [可變對象]
* [多變的]
* [變量參考]
* [變量數組]
* [可變參數]

### [函數](../syntax/04_function.md)

沒有副作用的子程序。

* [函數式編程](../syntax/23_scope.md#避免可變狀態函數式編程)

### 基本類型

###主格

通過名稱而不是對稱結構來區分。

* [命名類型] -> [類](../syntax/type/04_class.md)
* [報喜]
* [名義子類型](../syntax/type/05_nst_vs_sst.md)

### 捕獲-> [關閉]

### [協變]

在 Erg 中，如果 `T <: U` 則 `K(T) <: K(U)` 則稱 `K` 是協變的。

### [關鍵字參數]

`k` 以函數調用 `f(k: v)` 的形式出現。您可以通過形式參數名稱而不是按順序指定實際參數。

### 空集 -> [{}]

### 部分

* [區間類型](../syntax/type/11_interval.md)
* 區間運算符

### 嵌入式

Erg 標準 API 未在 .er 文件中實現。

### [類](../syntax/type/04_class.md)

具有繼承功能的結構/抽象數據類型。在 Erg 中，它是一種實現命名子類型化和覆蓋的類型。
在 Erg 中，模塊是模塊對象負責，類型是類型對象，而其他語言可能負責模塊和類型。

### [關閉]

### [全局變量]

### [克隆]

### [繼承](../syntax/type/07_inheritance.md)

定義一個類是另一個類的父類集。
繼承的類稱為超類，繼承的類稱為子類。
子類具有其超類的所有功能。

### 高樓層

* [高階種類](../syntax/type/advanced/kind.md)
* 高階類型
* 高階函數

### [公共變量]

### [結構子類型]

### ~~后向引用~~ -> [后向引用]

### [復制]

### 評論

### [集合](../syntax/10_array.md)

### 冒號 -> [:]

### [構造函數](../syntax/type/04_class.md)

### 容器

### 編譯器

### [編譯時計算](../syntax/04_function.md#compile-time function)

### 逗號 -> [,]

## sa線

### 遞歸

參考自己。

* 遞歸
* [遞歸函數](../syntax/04_function.md#遞歸函數)

### 下標 -> [索引]

### [子類型多態性](../syntax/type/overloading.md)

具有子類型的多態性。子類型對應于類型中的集合包含。

### 子程序

模塊化處理的對象。 Erg 中函數、過程和方法的通用術語。

### [參考](../syntax/18_memory_management.md#borrowed)

* 參考對象
* [引用計數 (RC)](../syntax/18_memory_management.md#memory management)
* 引用相等 -> [副作用](../syntax/07_side_effect.md)

### [標識符](../syntax/02_variable.md/# 賦值)

### 簽名

* 類型簽名

### [dict](../syntax/11_dict.md)

### 自然數 -> Nat

### 泛型 -> 泛型

### 發電機

### 投影類型

### 借用-> [參考](../syntax/18_memory_management.md#borrowed)

### [陰影](../syntax/02_name.md# variables)

通過在內部范圍內定義具有相同名稱的變量來覆蓋對變量的引用。

### kind -> [kind](../syntax/type/advanced/kind.md)

大致類型的類型。

### set -> set

在 Erg 中，它表示一個 Set 對象。

### 謂詞

* 謂詞函數

返回布爾類型的函數。

### 條件分支

### 所有權

對象唯一性的概念。
如果您擁有對象的所有權，則可以使用 mutable 參考它。

###  Boolean -> Bool

### 單例

從只能創建一個實例的類創建的實例。一種設計模式，可確保只創建一個類的一個實例。

### [Symbol] -> [Identifier](../syntax/02_name.md)

* 符號化

### [腳本](../syntax/00_basic.md# 腳本)

包含 Erg 程序的文件。

### 范圍

變量管理單元。外部作用域不能引用內部作用域中存在的變量。
當范圍退出時，引用計數為 0 的對象將被釋放。

### 擴展運算符 -> expansion assignment

### [切片](../syntax/10_array.md#slice)

表示數組子序列的對象，以 `x[a..b]` 的形式生成。

### 控制字符

### 整數 -> Int

一組自然數加上負數。

### [設置](../syntax/12_set.md)

### 分號 -> ;

### [聲明](../syntax/03_declaration.md)

顯式類型變量。

### 全名

* 通用類型 -> [多態類型](../syntax/type/quantified.md)
  * 封閉式通用
  * 打開通用
* 通用函數 -> 多相關函數
* 通用量化

### 前綴運算符

運算符 `°` 以 `°x` 的形式應用。

### 相互遞歸

### 下標 -> index

### 屬性

* 屬性子類型

## 塔線

### [代數](../syntax/02_name.md)

* [代數類型](../syntax/type/13_algebraic.md)
* 代數數據類型

### [賦值](../syntax/02_variable.md/#assignment)

### 多

* [多重繼承](../syntax/type/07_inheritance.md/#禁止多重繼承)
* 多重賦值
* 重載 -> [不重載]

### 多態性

* [多態類型](../syntax/type/quantified.md)
* 多相關系數

### 多態性 -> [多態性]

### 鴨子類型

### [元組](../syntax/11_tuple.md)

### 單相

* 單相
* 單相型
* 單相關系數

### [延遲初始化]

### 提取分配

### 抽象語法樹 -> [AST]

### 中綴運算符

運算符 `°` 以 `x°y` 的形式應用。

### [常數](../syntax/02_name.md/#constant)

不可變的，編譯時可評估的代數。

* [常量類型](../syntax/type/advanced/const.md)
* [常量表達式](../syntax/type/advanced/const.md)

### 定義

分配與變量對應的對象。

### 提供的屬性

可作為 API 使用的屬性。特別是由特征自動實現的屬性。

### 申請

將參數傳遞給函數對象并獲取評估結果。

### [裝飾器](../syntax/29_decorator.md)

``` python
@deco
f x = ...
```

語法糖，或“裝飾”。大致等于`_f x = ...; f = 裝飾 _f`。 `deco` 本身只是一個高階子程序。

### 析構函數

對象被銷毀時調用的方法。

### 程序 -> [procedure](../syntax/08_procedure.md)

讀取和寫入可變狀態的子程序。
有時會說程序的執行結果可以根據調用過程的順序而改變，但如果我們談論交換性，這是不正確的。
例如，作為函數子類型的運算符通常不可交換。

### [默認參數](../syntax/04_function.md/#default arguments default-parameters)

通過指定形式參數的默認值，可以在調用時省略實際參數的指定的函數。

### 擴張

* 擴展運算符
* 擴展分配

### [特殊格式](../syntax/../API/special.md)

不能作為實際參數傳遞的對象。

### 匿名函數 -> [anonymous function](../syntax/20_lambda.md)

由匿名函數運算符`->`創建的函數對象。可以在不定義名稱的情況下使用。

### 點運算符 (`.`) -> attribute reference

### 頂部

* 頂部類型 -> [結構對象]
* 頂級 -> [對象]

### [特征](../syntax/type/03_trait.md)

## na line

### [理解](../syntax/27_comprehension.md)

### ~~中綴運算符~~ -> 中綴運算符

### 命名空間

## 是一行

### [數組](../syntax/10_array.md)

### [派生類型](../syntax/type/variances.md/# 用戶定義的類型變體)

### [模式(匹配)](../syntax/26_pattern_matching.md)

### [包](../syntax/33_package_system.md)

### hashmap -> [dict](../syntax/11_dict.md)

### [補丁](../syntax/type/07_patch.md)

### 公共變量-> [public variables](../syntax/19_visibility.md)

### 參數 -> [argument](../syntax/04_function.md)

### [參數多態](../syntax/type/overloading.md)

### [逆變](../syntax/type/advanced/variance.md)

### 相比

* 比較運算符
* 可比類型

### [私有變量](../syntax/19_visibility.md)

### 標準

* 標準輸出
* 標準輸入
* 標準庫

### [副作用](../syntax/07_side_effect.md)

代碼應該/不應該讀/寫外部可變狀態。

### 復數 -> 復數

### 浮動 -> 浮動

### 私有變量 -> 私有變量

### 布爾代數-> Bool

### [程序](../syntax/08_procedure.md)

### [參數](../syntax/04_function.md)

### 部分類型 -> Subtyping

### [不可變]

在 Erg 中，一個對象永遠不應該改變它的內容。

* [不可變對象]
* [不可變類型]
* [不可變引用]

### [篩子類型](../syntax/type/12_refinement.md)

### [堵塞]

### 解構賦值

### [變量](../syntax/02_variable.md)

### 底部

* 底部類型 -> [{}]
* 底層 -> [從不]

### [多態性]

## ma line

### ~~ 前綴運算符 ~~ -> 前綴運算符

### [標記類型](../syntax/type/advanced/marker_trait.md)

### [匿名函數](../syntax/21_lambda.md)

### 可變 -> [可變]

### [移動]

### 方法

### 元字符

### [模塊](../syntax/24_module.md)

### [字符串] -> [字符串]

* [字符串插值](../syntax/01_literal.md/#Str 字面量)

### 返回值

## 或行

### [幻像類型](../syntax/type/advanced/phantom.md)

### 請求屬性

### [元素]

### [稱呼]

## 拉線

### [圖書館]

### lambda 表達式 -> [匿名函數](../syntax/20_lambda.md)

### 排名

* [rank2 多態性](../syntax/type/advanced/rank2type.md)

### [文字](../syntax/01_literal.md)

* [文字標識符](../syntax/18_naming_rule.md/#literal identifier)

### [量化](../syntax/type/quantified.md)

### [布局](../syntax/type/mut.md)

### [枚舉](../syntax/type/10_enum.md)

### [記錄](../syntax/12_record.md)

* [記錄類型]
* 記錄多態 -> Column Polymorphism

### 列多態

### [局部變量](../syntax/19_visibility.md)

## 線

### 通配符
