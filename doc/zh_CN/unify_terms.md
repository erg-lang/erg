# 术语统一

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/unify_terms.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/unify_terms.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## 可访问性，可见性

使用可见性

## 类型绑定，类型约束

给定量化和细化类型的谓词表达式列表。使用类型边界

## 子程序、例程、子程序

使用子程序

## 引用透明/不透明，有/没有副作用

使用有/无副作用

## 标识符、代数、变量、名称、符号

就其本义而言，

* 符号: 在源代码中实心编写的字符(符号、控制字符等除外)，不是字符串对象(未包含在""中)。符号在 Ruby、Lisp 等中作为原始类型存在，但在 Erg 中它们不被视为对象
* 标识符: (并且可以)引用某个对象的符号，而不是保留字。例如，在 Python 中 class 和 def 不能用作标识符。由于 Erg 没有保留字，所以除了某些符号外，所有符号都可以用作标识符
* 名称: 与标识符的含义几乎相同。它有时与 Erg 中的代数同义使用
* 代数名称: 相当于Erg中的标识符。在 C 中，函数名称是标识符，而不是代数名称。"代数"指的是语言特性本身，它允许您使用 `=`(变量赋值运算符)或 `=`(常量赋值运算符)来分配对象

```python
代数名称<: (名称==标识符)​​<: 符号
变量 + 常数 == 代数
```

然而，应该称为"代数"的东西，往往被称为"变量"。这就是数学术语的效果
值内容可以改变的变量是可变变量，值内容不改变的变量是不可变变量
请注意，常量始终是不可变的

Erg 中不使用代数名称和名称，使用统一标识符
但是，一般来说，具有 `v = 1` 的 `v` 称为"变量 v"，而具有 `C = 1` 的 `C` 称为"常量 C"。.

## 属性、字段、属性

使用属性。顺便说一句，记录是一个函数，它可以定义一个没有类的具有元素属性的对象

## 应用程序，调用

为子例程对象提供参数并获得结果
使用呼叫。这是因为Application有"应用软件"的用法

## 数组列表

使用数组。Erg 数组(通常)在内存中是连续的
List 是指所谓的链表，或者说列表作为 Python 数据类型

## lambda 函数、lambda 表达式、匿名函数

与匿名函数统一。在英文中，可以使用 Lambda 来缩短字符数，但正式名称是 Anonymous function。