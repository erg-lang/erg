# 内置 Erg 类型列表

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

类型本身的属性不存储在 `.__dict__` 中，不能从实例中引用

## 基本类型

### 对象

* `__dir__`：将对象的属性作为数组返回(dir函数)
* `__getattribute__`: 获取并返回一个属性
* `__hash__`：返回对象的哈希值
* `__repr__`：对象的字符串表示(不存在丰富/默认实现)
* `__sizeof__`：返回对象的大小(包括在堆中分配的大小)

### 显示

* `__str__`：返回对象的字符串表示(丰富)

### Fmt

* `__format__`: 返回一个格式化的字符串

### 文档

* `__doc__`：对象描述

### 命名

* `__name__`: 对象的名称

### 泡菜

* `__reduce__`: 用 Pickle 序列化对象
* `__reduce_ex__`: __reduce__ 允许你指定协议版本

## 对象系统

Trait 类相当于 Python 中的 ABC(抽象基类，接口)
实例属于1、True、"aaa"等。
类是 Int、Bool、Str 等。

### 类型

* `__父类__`：超类型(`__mro__` 是一个数组，但这个是一个 Set)
* `__basicsize__`:
* `__dictoffset__`：Evm 不支持
* `__flags__`:
* `__itemsize__`：实例的大小(如果不是类，则为 0)
* `__weakrefoffset__`：Evm 不支持
* `__membercheck__`: 相当于`ismember(x, T)`
* `__subtypecheck__`：等价于`issubtype(U, T)`，别名`__subclasshook__`(兼容CPython)

### 实例

* `__class__`：返回创建实例的类(自动附加到使用 `.new` 创建的对象)

### Class

* `__mro__`：用于方法解析的类型数组(包括自身，始终以 Object 结尾)
* `__base__`：基本类型(`__mro__[1]` 如果有多个)
* `__new__`: 实例化
* `__init__`: 初始化实例
* `__init_subclass__`: 初始化实例
* `__intstancecheck__`：使用类似于 `MyClass.__instancecheck__(x)`，等价于 `isinstance(x, MyClass)`
* `__subclasscheck__`：等价于 `issubclass(C, MyClass)`

## 运算符

此处指定以外的运算符没有特殊类型

### 方程

* `__eq__(self, rhs: Self) -> Bool`: 对象比较函数 (==)
* `__ne__`: 对象比较函数 (!=)，默认实现

### 秩序

* `__lt__(self, rhs: Self) -> Bool`: 对象比较函数 (<)
* `__le__`：对象比较函数(<=)，默认实现
* `__gt__`：对象比较函数(>)，默认实现
* `__ge__`：对象比较函数(>=)，默认实现

### BinAdd

* 实现 `__add__(self, rhs: Self) -> Self`: `+`

### 添加R

* `__add__(self, rhs: R) -> Self.AddO`

### Sub R

* `__sub__(self, rhs: R) -> Self.SubO`

### Mul R

* `__mul__(self, rhs: R) -> Self.MulO`

### BinMul <: Mul Self

* `__pow__`：实现 `**`(默认实现)

### Div R, O

* 实现 `__div__(self, rhs: Self) -> Self`: `/`，可能会因为 0 而恐慌

### BinDiv <: Div Self

* `__mod__`: 实现 `%` (默认实现)

## 数值型

### Num (= Add and Sub and Mul and Eq)

例如，除了Complex，Vector、Matrix和Tensor都是Num(Matrix和Tensor中的*分别与dot和product相同)

### Complex (= Inherit(Object, Impl := Num))

* `imag: Ratio`：返回虚部
* `real: Ratio`：返回实部
* `conjugate self -> Complex`：返回复共轭

### Float (= Inherit(FloatComplex, Impl := Num))

### Ratio (= Inherit(Complex, Impl := Num))

* `numerator: Int`: 返回分子
* `denominator: Int`: 返回分母

### Int (= Inherit Ratio)

### Nat (= Inherit Int)

* `times!`: 运行 proc self 时间

## 其他基本类型

### 布尔值

* `__and__`:
* `__or__`:
* `not`:

## 字符串 (<: 序列)

* `capitalize`
* `chomp`: 删除换行符
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

* `from_bytes`：从字节转换
* `to_bytes`：转换为字节(指定长度和字节序(字节序))
* `bit_length`：返回位长度

### 可迭代 T

请注意，它不是 `Iterator` 本身的类型。 `Nat` 是 `Iterable` 但你不能 `Nat.next()`，你需要 `Nat.iter().next()`。

* `iter`：创建一个迭代器。

### 迭代器 T

Nat 和 Range 有迭代器，所以 `Nat.iter().map n -> n**2`, `(3..10).iter().fold (sum, n) -> sum + n*2`等是可能的。
由于所有和任何在使用后都会被破坏，因此没有副作用。这些应该使用没有副作用的 `next` 来实现，但内部使用 `Iterator!.next!` 来提高执行效率。

* `next`：返回第一个元素和剩余的迭代器。
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

* `next!`：获取第一个元素。

## SizedIterator T = 迭代器 T 和 ...

有限数量元素的迭代器。

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

* `concat`: 合并两个 Seq
* `__getitem__`：等同于使用 `[]` 访问(否则会出现恐慌)
* 与 `get`: __getitem__ 不同，它返回 Option
* `maketrans`：创建替换表(静态方法)
* `replace`: 替换
* `translate`：根据替换表替换
* `insert`: 添加到 idx
* `remove`: 删除 idx
* `prepend`: 前置
* `dequeue`: 移除头部
* `push`：添加到末尾
* `pop`: 取尾巴
* `dedup`：删除连续值
* `uniq`：删除重复元素(通过 sort |> dedup 实现，因此顺序可能会改变)
* `swap`：交换元素
* `reverse`：反转元素
* `sort`: 排序元素
* `first`:
* `last`:

### Seq!T (= Seq T and ...)

* `__setitem__!`:
* `__delitem__!`:
* `插入！`：添加到 idx
* `remove!`: 删除 idx
* `prepend!`：前置
* `dequeue!`: 删除开头
* `push!`：添加到末尾
* `pop!`：拿尾巴
* `dedup!`：删除连续值
* `uniq!`: 删除重复元素(通过排序实现！|> dedup!，因此顺序可能会改变)
* `swap!`：交换元素
* `reverse!`：反转元素
* `set!`
* `sort!`: 排序元素
* `translate!`