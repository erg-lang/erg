# 術語詞典

## 符號

### &excl;

過程或附加在標識符末尾的標記，以指示其為可變類型。或者變量運算符。

### ../syntax/00_basic.md/# 註釋

### $

### %

### &

### ′(single quote)

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

### [秩 1 多相]

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

### ref!

### [Result]

### [rootobj]

## S

### self

### [Self](../syntax/type/special.md)

### [side-effect](../syntax/07_side_effect.md)

### [Str]

## T

### Trait

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

## 阿行

### 斷言

檢查代碼中的條件是否成立（通常是在運行時）。使用函數等進行操作。


```erg
sum = !0
for! 0..10, i =>
    sum.add! i

assert sum == 55
```

### 值對象

在 Erg 中，與基本對象相同。編譯時可以進行評價，擁有不言而喻的比較方法。

### 附著面片../syntax/29_decorate.md#attach

為 Tracet 提供標準實現的補丁程序。

### 即席多相->

所謂超載的多相。

### 屬性-屬性

標識符中的<gtr=“9”/>部分。

### 安利

運算符使用多少個操作數。

### 依賴關係../syntax/type/dependent_type.md

以值（通常為非類型）為參數的類型。

### 可變體-> 不可變

表示目標保持不變。在其他語言中，變量也具有可變/可變特性，但在 Erg 中，變量都是可變的。

### 參數-> 參數

### 實例

類創建的對象。類類型的元素。

### 即時塊（../syntax/00_basic.md# 表達式分隔符）


```erg
x =
    y = f(a)
    z = g(b, c)
    y + z
```

### 索引

形式為，或其中<gtr=“11”/>的部分。稱為 Indexable 對象。

### 縮進../syntax/00_basic.md# 縮進

靠空格使句子向右靠。縮進。 Erg 通過縮進來表現塊。這叫做越位規則。

### 別名

別名。

### 錯誤

規範規定的異常狀態。

* [エラーハンドリング]

### 運算符../syntax/06_operator.md

將運算應用於操作數的對象。或表示對象的符號。

* [演算子の結合強度]

### 覆蓋

用子類覆蓋超類的方法。在 Erg 中，覆蓋時必須安裝裝飾器。

### 禁止過載（../syntax/type/overloading.md）

### 越位規則->

### 對象

* 面向對象

### 操作數->

### 操作員->

## 家行

### 卡印（../syntax/type/advanced/kind.md）

所謂模子的模子。

### 可見性

標識符是否可從外部（範圍外或單獨模塊、單獨軟件包）引用的性質。

### 類型

要對項進行分組的對象。

* [型指定]
* 清除類型（../syntax/type/advanced/erasure.md）
* [型推論]
* 類型註釋../syntax/type/conv_type.md
* [型引數]
* 添加類型（../syntax/type/advanced/erasure.md）
* 類型變量（../syntax/type/type_variable.md）
* [型製約]

### 保護

### 封裝

隱藏實現細節。

### 變量

不可變。

* [可変オブジェクト]
* [可変型]
* [可変參照]
* [可変配列]
* [可変長引數]

### 函數../syntax/04_function.md

沒有副作用的子程序。

* 函數型編程（../syntax/23_scop.md# 避免變量狀態函數型編程）

### 基本類型

### 記名的

通過名稱而不是對稱結構來區分。

* [記名型]->
* [記名化]
* 記名部分類型../syntax/type/05_nst_vs_sst.md

### 捕捉-> 閉包

### 協變

在 Erg 中，當時，如果<gtr=“19”/>，則<gtr=“20”/>為協變。

### 關鍵字參數

函數調用形式中的<gtr=“22”/>。實際自變量可以用假自變量名而不是順序指定。

### 空集->[{}]

### 區間

* 間隔類型（../syntax/type/11_interval.md）
* 區間運算符

### 嵌入

未在.er 文件中實現的 Erg 標準 API。

### 類../syntax/type/04_class.md

具有繼承功能的結構和抽像數據類型。在 Erg 中是為了實現記名式分型以及覆蓋的類型。在其他語言中也有承擔模塊和型的責任和義務的情況，在 Erg 中，模塊是模塊對象，型是型對象承擔其責任和義務。

### 閉合

### 全局變量

### 克隆

### 繼承

定義以某個類為上級集合的類。繼承源的類稱為超類，繼承目標的類稱為子類。子類具有超類的所有功能。

### 高階

* 高階../syntax/type/advanced/kind.md
* 高階型
* 高階函數

### 公共變量

### 結構子類型

### ~~ 向後參照 ~~~->[向前參照]

### 複製

### 註釋

### 集合../syntax/10_array.md

### 冒號->[：]

### 構造函數（../syntax/type/04_class.md）

### 集裝箱

### 編譯器

### 編譯時計算../syntax/04_function.md# 編譯時函數

### 逗號->[，]

## 差行

### 遞歸

指自己。

* 遞歸型
* 遞歸函數../syntax/04_function.md# 遞歸函數

### 下標-> 索引

### 多相子類型（../syntax/type/overloading.md）

多相分型。子類型是指在類型中與集合的包含關係相對應的類型。

### 子程序

模塊化處理的對象。 Erg 中函數、過程和方法的通用名稱。

### 參考（../syntax/18_memory_management.md# 借用）

* 引用對象
* 參照計數 (RC) （../syntax/18_memory_management.md# 內存管理）
* 參考等效性->

### 標識符（../syntax/02_variable.md/# 賦值）

### 簽名

* 類型簽名

### 詞典../syntax/11_dict.md

### 自然數->Nat

### 通用->[全稱類型]

### 發電機

### 投影類型

### 借用->

### 陰影（../syntax/02_name.md# 變量）

在內部作用域中定義一個同名的變量，並覆蓋該變量的引用。

### 種子->

大致是個模子。

### 集-> 集

在 Erg 中是 Set 對象。

### 謂語

* [述語関數]

返回布爾類型的函數。

### 條件分歧

### 所有權

關於對象唯一性的概念。如果擁有對象的所有權，則可以對對象進行可變引用。

### 真偽類型-> 布爾

### 單噸

從只能生成一個實例的類生成的實例。也指確保只生成一個類實例的設計模式。

### 符號->

* [シンボル化]

### 腳本../syntax/00_basic.md# 腳本

描述 Erg 程序的文件。

### 範圍

變量管理中的單位。外側的範圍不能參照存在於內側範圍的變量。另外，脫離範圍時，參照點數為 0 的對像被釋放。

### 跨頁運算符-> 展開賦值

### 切片../syntax/10_array.md# 切片

以形式生成的表示數組子串的對象。

### 控製字符

### 整數-> 輸入

自然數加負數的集合。

### 集../syntax/12_set.md

### 分號->[；]

### 聲明../syntax/03_declaration.md

顯式設置變量類型。

### 全稱

* 全稱類型->
  * 封閉全稱類型
  * 打開的全稱類型
* 全稱函數-> 多相關數
* 全稱量化

### 前綴運算符

以格式應用的運算符<gtr=“30”/>。

### 互相的遞歸

### 下標-> 索引

### 屬性

* [屬性的部分型]

## 多行

### 代數../syntax/02_name.md

* 代數類型（../syntax/type/13_algebraic.md）
* 代數數據類型

### 賦值../syntax/02_variable.md/# 賦值

### 多重

* 多重繼承（../syntax/type/07_inheritance.md/# 禁止多重繼承）
* 多重賦值
* 多重定義-> 禁止過載

### 多相

* 多相類型（../syntax/type/quantified.md）
* 多相關數

### 多態-> 多態

### 烤鴨打字

### 元組（../syntax/11_tuple.md）

### 單相

* 單相化
* 單相型
* 單相關數

### 延遲初始化

### 抽出賦值

### 抽象語法樹->[AST]

### 中置運算符

以格式應用的運算符。

### 常量../syntax/02_name.md/# 常量

可執行的、編譯時可評估的代數。

* 常量類型（../syntax/type/advanced/const.md）
* 常量表達式（../syntax/type/advanced/const.md）

### 定義

分配與變量對應的對象。

### 授課屬性

可用作 API 的屬性。特別是由trait自動實現的屬性。

### 應用

將參數傳遞給函數對像以獲得評估結果。

### 裝飾器../syntax/29_decorate.md


```erg
@deco
f x = ...
```

的语法糖，或者。大約等於。本身只是一個高階子程序。

### 析構

銷毀對象時調用的方法。

### 過程->

讀取和寫入可變狀態的子程序。有時會解釋程序根據調用順序的不同，程序的執行結果也會發生變化，但如果說的是可換性的話，這是錯誤的。例如，作為函數子類型的運算符一般不是可換的。

### 缺省參數../syntax/04_function.md/# 缺省參數 default-parameters

通過為虛擬自變量指定缺省值，調用時可以省略實際自變量指定的功能。

### 展開

* [展開演算子]
* [展開代入]

### 特殊格式（../syntax/../API/special.md）

不能傳遞給實際參數的對象。

### 匿名函數->

由未命名函數運算符生成的函數對象。不用定義名字就能使用。

### 點運算符（）->[屬性引用]

### 頂部

* 頂部類型-> 結構對象
* 頂級-> 對象

### TRAIT（../syntax/type/03_trait.md）

## 標題

### 內涵符號../syntax/27_comprehension.md

### 中置算子 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### 名稱空間

## 派系

### 陣列../syntax/10_array.md

### 派生類型（../syntax/type/variances.md/# 用戶定義類型的退化）

### 圖案（匹配）../syntax/26_pattern_matching.md

### 軟件包../syntax/33_package_system.md

### 哈希映射->

### 面片../syntax/type/07_patch.md

### 公共變量->

### 參數->

### 參數化多相（../syntax/type/overloading.md）

### 反變（../syntax/type/advanced/variance.md）

### 比較

* [比較演算子]
* [比較可能型]

### 私有變量../syntax/19_visibility.md

### 標準

* 標準輸出
* 標準輸入
* 標準庫

### 副作用../syntax/07_side_effect.md

代碼不能讀取或寫入外部可變狀態。

### 複數->

### 浮點數-> 浮點

### 專用變量-> 專用變量

### 布爾代數-> 布爾

### 程序../syntax/08_procedure.md

### 參數（../syntax/04_function.md）

### 部分類型-> 子類型

### 不變

在 Erg 中，對像不改變其內容。

* [不変オブジェクト]
* [不変型]
* [不変參照]

### 篩型（../syntax/type/12_refinement.md）

### 塊

### 分解賦值

### 變量../syntax/02_variable.md

### 底部

* 底部->[{}]
* 底部類->Never

### 多態

## 真行

### 前綴運算符 ~~~~~~ 前綴運算符

### 標記類型../syntax/type/advanced/marker_trait.md

### 無名函數../syntax/21_lambda.md

### 可變-> 可變

### 移動

### 方法

### 元字符

### 模塊（../syntax/24_module.md）

### 字符串->Str

* 字符串插值（../syntax/01_literal.md/#Str 文字）

### 返回值

## 夜行

### 幽靈類型（../syntax/type/advanced/phantom.md）

### 請求屬性

### 元素

### 調用

## 羅列

### 庫

### 拉姆達公式->

### 等級

* 通道 2 多相../syntax/type/advanced/rank2type.md

### 文字（../syntax/01_literal.md）

* 文字標識符（../syntax/18_naming_rule.md/# 文字標識符）

### 量化（../syntax/type/quantified.md）

### 佈局（../syntax/type/mut.md）

### 枚舉類型（../syntax/type/10_enum.md）

### 記錄../syntax/12_record.md

* [レコード型]
* 記錄多相-> 列多相

### 列多相

### 局部變量../syntax/19_visibility.md

## 和行

### 通配符