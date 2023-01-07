# 內置 Erg 類型列表

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

類型本身的屬性不存儲在 `.__dict__` 中，不能從實例中引用

## 基本類型

### 對象

* `__dir__`: 將對象的屬性作為數組返回(dir函數)
* `__getattribute__`: 獲取并返回一個屬性
* `__hash__`: 返回對象的哈希值
* `__repr__`: 對象的字符串表示(不存在豐富/默認實現)
* `__sizeof__`: 返回對象的大小(包括在堆中分配的大小)

### 顯示

* `__str__`: 返回對象的字符串表示(豐富)

### Fmt

* `__format__`: 返回一個格式化的字符串

### 文檔

* `__doc__`: 對象描述

### 命名

* `__name__`: 對象的名稱

### 泡菜

* `__reduce__`: 用 Pickle 序列化對象
* `__reduce_ex__`: __reduce__ 允許你指定協議版本

## 對象系統

Trait 類相當于 Python 中的 ABC(抽象基類，trait)
實例屬于1、True、"aaa"等
類是 Int、Bool、Str 等

### 類型

* `__父類__`: 父類型(`__mro__` 是一個數組，但這個是一個 Set)
* `__basicsize__`:
* `__dictoffset__`: Evm 不支持
* `__flags__`:
* `__itemsize__`: 實例的大小(如果不是類，則為 0)
* `__weakrefoffset__`: Evm 不支持
* `__membercheck__`: 相當于`ismember(x, T)`
* `__subtypecheck__`: 等價于`issubtype(U, T)`，別名`__subclasshook__`(兼容CPython)

### 實例

* `__class__`: 返回創建實例的類(自動附加到使用 `.new` 創建的對象)

### Class

* `__mro__`: 用于方法解析的類型數組(包括自身，始終以 Object 結尾)
* `__base__`: 基本類型(`__mro__[1]` 如果有多個)
* `__new__`: 實例化
* `__init__`: 初始化實例
* `__init_subclass__`: 初始化實例
* `__intstancecheck__`: 使用類似于 `MyClass.__instancecheck__(x)`，等價于 `isinstance(x, MyClass)`
* `__subclasscheck__`: 等價于 `issubclass(C, MyClass)`

## 運算符

此處指定以外的運算符沒有特殊類型

### 方程

* `__eq__(self, rhs: Self) -> Bool`: 對象比較函數 (==)
* `__ne__`: 對象比較函數 (!=)，默認實現

### 秩序

* `__lt__(self, rhs: Self) -> Bool`: 對象比較函數 (<)
* `__le__`: 對象比較函數(<=)，默認實現
* `__gt__`: 對象比較函數(>)，默認實現
* `__ge__`: 對象比較函數(>=)，默認實現

### BinAdd

* 實現 `__add__(self, rhs: Self) -> Self`: `+`

### 添加R

* `__add__(self, rhs: R) -> Self.AddO`

### Sub R

* `__sub__(self, rhs: R) -> Self.SubO`

### Mul R

* `__mul__(self, rhs: R) -> Self.MulO`

### BinMul <: Mul Self

* `__pow__`: 實現 `**`(默認實現)

### Div R, O

* 實現 `__div__(self, rhs: Self) -> Self`: `/`，可能會因為 0 而恐慌

### BinDiv <: Div Self

* `__mod__`: 實現 `%` (默認實現)

## 數值型

### Num (= Add and Sub and Mul and Eq)

例如，除了Complex，Vector、Matrix和Tensor都是Num(Matrix和Tensor中的*分別與dot和product相同)

### Complex (= Inherit(Object, Impl := Num))

* `imag: Ratio`: 返回虛部
* `real: Ratio`: 返回實部
* `conjugate self -> Complex`: 返回復共軛

### Float (= Inherit(FloatComplex, Impl := Num))

### Ratio (= Inherit(Complex, Impl := Num))

* `numerator: Int`: 返回分子
* `denominator: Int`: 返回分母

### Int (= Inherit Ratio)

### Nat (= Inherit Int)

* `times!`: 運行 proc self 時間

## 其他基本類型

### 布爾值

* `__and__`:
* `__or__`:
* `not`:

## 字符串 (<: 序列)

* `capitalize`
* `chomp`: 刪除換行符
* `isalnum`:
* `isascii`:
* `isalpha`:
* `isdecimal`:
* `isdight`:
* `isidentifier`
* `islower`
* `isnumeric`
* `isprintable`
* `isspace`
* `istitle`
* `isupper`
* `lower`
* `swapcase`
* `title`
* `upper`

## 其他

### 位

* `from_bytes`: 從字節轉換
* `to_bytes`: 轉換為字節(指定長度和字節序(字節序))
* `bit_length`: 返回位長度

### 可迭代 T

請注意，它不是 `Iterator` 本身的類型。`Nat` 是 `Iterable` 但你不能 `Nat.next()`，你需要 `Nat.iter().next()`

* `iter`: 創建一個迭代器

### 迭代器 T

Nat 和 Range 有迭代器，所以 `Nat.iter().map n -> n**2`, `(3..10).iter().fold (sum, n) -> sum + n*2`等是可能的
由于所有和任何在使用后都會被破壞，因此沒有副作用。這些應該使用沒有副作用的 `next` 來實現，但內部使用 `Iterator!.next!` 來提高執行效率

* `next`: 返回第一個元素和剩余的迭代器
* `all`
* `any`
* `filter`
* `filter_map`
* `find`
* `find_map`
* `flat_map`
* `flatten`
* `fold`
* `for_each`
* `map`
* `map_while`
* `nth`
* `pos`
* `take`
* `unzip`
* `zip`

### Iterator!T = IteratorT 和 ...

* `next!`: 獲取第一個元素

## SizedIterator T = 迭代器 T 和 ...

有限數量元素的迭代器

* `len`:
* `chain`:
* `count`:
* `is_empty`:
* `rev`:
* `next_back`:
* `nth_back`:
* `rfind`:
* `rfold`:
* `sum`:
* `max`:
* `min`:

## Seq T = SizedIterable T 和 ...

* `concat`: 合并兩個 Seq
* `__getitem__`: 等同于使用 `[]` 訪問(否則會出現恐慌)
* 與 `get`: __getitem__ 不同，它返回 Option
* `maketrans`: 創建替換表(靜態方法)
* `replace`: 替換
* `translate`: 根據替換表替換
* `insert`: 添加到 idx
* `remove`: 刪除 idx
* `prepend`: 前置
* `dequeue`: 移除頭部
* `push`: 添加到末尾
* `pop`: 取尾巴
* `dedup`: 刪除連續值
* `uniq`: 刪除重復元素(通過 sort |> dedup 實現，因此順序可能會改變)
* `swap`: 交換元素
* `reverse`: 反轉元素
* `sort`: 排序元素
* `first`:
* `last`:

### Seq!T (= Seq T and ...)

* `__setitem__!`:
* `__delitem__!`:
* `插入！`: 添加到 idx
* `remove!`: 刪除 idx
* `prepend!`: 前置
* `dequeue!`: 刪除開頭
* `push!`: 添加到末尾
* `pop!`: 拿尾巴
* `dedup!`: 刪除連續值
* `uniq!`: 刪除重復元素(通過排序實現！|> dedup!，因此順序可能會改變)
* `swap!`: 交換元素
* `reverse!`: 反轉元素
* `set!`
* `sort!`: 排序元素
* `translate!`