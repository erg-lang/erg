# 词汇表

## 象征

### &excl;

添加到标识符末尾的标记以指示它是过程，变量类型或变异运算符。

### [&#35;](../syntax/00_basic.md/# comment)

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

检查（通常在运行时）代码中的条件是否为真。 这是使用 `assert` 函数等来完成的。

```python
sum = !0
for! 0..10, i =>
    sum.add!i

assert sum == 55
```

### 值对象

在 Erg 中，相当于基础对象。 它可以在编译时进行评估，并且具有简单的比较方法。

### [附件补丁](../syntax/29_decorator.md#attach)

为特征提供标准实现的补丁。

### Ad hoc 多态性 -> [无重载](../syntax/type/overloading.md)

具有所谓重载的多态性。

### 属性-> [属性]

`x.y` 标识符中的 `y` 部分。

### Arity

运算符需要多少个操作数。

### [依赖类型](../syntax/type/dependent_type.md)

参数是值的类型（习惯上说，不是类型）。

### 不可变 -> [不可变]

表示目标不会改变。
其他语言中的变量也是不可变/可变的，但是在 Erg 中所有的变量都是不可变的。

### 参数 -> [参数]

### 实例

由类创建的对象。 类类型的元素。

### [即时封锁](../syntax/00_basic.md#expression separator)

```python
x =
    y = f(a)
    z = g(b,c)
    y+z
```

### 指数

形式为“x[i]”，或其“i”部分。我们称 `x` 为 Indexable 对象。

### [缩进](../syntax/00_basic.md#indent)

通过向空格移动将文本向右对齐。缩进。
Ergs 通过缩进表示块。这称为越位规则。

### 别名

别名。

### 错误

规范中定义的异常情况。

* [错误处理]

### [运算符](../syntax/06_operator.md)

将操作应用于其操作数的对象。或表示该对象的符号。

* [运算符绑定强度]

### 覆盖

在子类中覆盖超类方法。
在 Erg 中，您必须在覆盖时添加 `Override` 装饰器。

### [不重载](../syntax/type/overloading.md)

### 越位规则-> [缩进](../syntax/00_basic.md#indent)

### [目的]

* 面向对象

### 操作数 -> [操作数](../syntax/06_operator.md)

### 运算符 -> [运算符](../syntax/06_operator.md)

##嘉线

### [种类](../syntax/type/advanced/kind.md)

所谓类型的类型。

### [可见性]

标识符是否可以被外部引用（超出范围，或在另一个模块或包中）的属性。

### [类型]

对术语进行分组的对象。

* [类型规格]
* [类型擦除](../syntax/type/advanced/erasure.md)
* [类型推断]
* [类型注释](../syntax/type/conv_type.md)
* [类型参数]
* [类型添加](../syntax/type/advanced/erasure.md)
* [类型变量](../syntax/type/type_variable.md)
* [类型约束]

### 监视

### 封装

隐藏实现细节。

### [多变的]

不能是一成不变的。

* [可变对象]
* [多变的]
* [变量参考]
* [变量数组]
* [可变参数]

### [函数](../syntax/04_function.md)

没有副作用的子程序。

* [函数式编程](../syntax/23_scope.md#避免可变状态函数式编程)

### 基本类型

###主格

通过名称而不是对称结构来区分。

* [命名类型] -> [类](../syntax/type/04_class.md)
* [报喜]
* [名义子类型](../syntax/type/05_nst_vs_sst.md)

### 捕获-> [关闭]

### [协变]

在 Erg 中，如果 `T <: U` 则 `K(T) <: K(U)` 则称 `K` 是协变的。

### [关键字参数]

`k` 以函数调用 `f(k: v)` 的形式出现。您可以通过形式参数名称而不是按顺序指定实际参数。

### 空集 -> [{}]

### 部分

* [区间类型](../syntax/type/11_interval.md)
* 区间运算符

### 嵌入式

Erg 标准 API 未在 .er 文件中实现。

### [类](../syntax/type/04_class.md)

具有继承功能的结构/抽象数据类型。在 Erg 中，它是一种实现命名子类型化和覆盖的类型。
在 Erg 中，模块是模块对象负责，类型是类型对象，而其他语言可能负责模块和类型。

### [关闭]

### [全局变量]

### [克隆]

### [继承](../syntax/type/07_inheritance.md)

定义一个类是另一个类的父类集。
继承的类称为超类，继承的类称为子类。
子类具有其超类的所有功能。

### 高楼层

* [高阶种类](../syntax/type/advanced/kind.md)
* 高阶类型
* 高阶函数

### [公共变量]

### [结构子类型]

### ~~后向引用~~ -> [后向引用]

### [复制]

### 评论

### [集合](../syntax/10_array.md)

### 冒号 -> [:]

### [构造函数](../syntax/type/04_class.md)

### 容器

### 编译器

### [编译时计算](../syntax/04_function.md#compile-time function)

### 逗号 -> [,]

## sa线

### 递归

参考自己。

* 递归
* [递归函数](../syntax/04_function.md#递归函数)

### 下标 -> [索引]

### [子类型多态性](../syntax/type/overloading.md)

具有子类型的多态性。子类型对应于类型中的集合包含。

### 子程序

模块化处理的对象。 Erg 中函数、过程和方法的通用术语。

### [参考](../syntax/18_memory_management.md#borrowed)

* 参考对象
* [引用计数 (RC)](../syntax/18_memory_management.md#memory management)
* 引用相等 -> [副作用](../syntax/07_side_effect.md)

### [标识符](../syntax/02_variable.md/# 赋值)

### 签名

* 类型签名

### [dict](../syntax/11_dict.md)

### 自然数 -> Nat

### 泛型 -> 泛型

### 发电机

### 投影类型

### 借用-> [参考](../syntax/18_memory_management.md#borrowed)

### [阴影](../syntax/02_name.md# variables)

通过在内部范围内定义具有相同名称的变量来覆盖对变量的引用。

### kind -> [kind](../syntax/type/advanced/kind.md)

大致类型的类型。

### set -> set

在 Erg 中，它表示一个 Set 对象。

### 谓词

* 谓词函数

返回布尔类型的函数。

### 条件分支

### 所有权

对象唯一性的概念。
如果您拥有对象的所有权，则可以使用 mutable 参考它。

###  Boolean -> Bool

### 单例

从只能创建一个实例的类创建的实例。一种设计模式，可确保只创建一个类的一个实例。

### [Symbol] -> [Identifier](../syntax/02_name.md)

* 符号化

### [脚本](../syntax/00_basic.md# 脚本)

包含 Erg 程序的文件。

### 范围

变量管理单元。外部作用域不能引用内部作用域中存在的变量。
当范围退出时，引用计数为 0 的对象将被释放。

### 扩展运算符 -> expansion assignment

### [切片](../syntax/10_array.md#slice)

表示数组子序列的对象，以 `x[a..b]` 的形式生成。

### 控制字符

### 整数 -> Int

一组自然数加上负数。

### [设置](../syntax/12_set.md)

### 分号 -> ;

### [声明](../syntax/03_declaration.md)

显式类型变量。

### 全名

* 通用类型 -> [多态类型](../syntax/type/quantified.md)
  * 封闭式通用
  * 打开通用
* 通用函数 -> 多相关函数
* 通用量化

### 前缀运算符

运算符 `∘` 以 `∘x` 的形式应用。

### 相互递归

### 下标 -> index

### 属性

* 属性子类型

## 塔线

### [代数](../syntax/02_name.md)

* [代数类型](../syntax/type/13_algebraic.md)
* 代数数据类型

### [赋值](../syntax/02_variable.md/#assignment)

＃＃＃ 多

* [多重继承](../syntax/type/07_inheritance.md/#禁止多重继承)
* 多重赋值
* 重载 -> [不重载]

### 多态性

* [多态类型](../syntax/type/quantified.md)
* 多相关系数

### 多态性 -> [多态性]

### 鸭子类型

### [元组](../syntax/11_tuple.md)

### 单相

* 单相
* 单相型
* 单相关系数

### [延迟初始化]

### 提取分配

### 抽象语法树 -> [AST]

### 中缀运算符

运算符 `∘` 以 `x∘y` 的形式应用。

### [常数](../syntax/02_name.md/#constant)

不可变的，编译时可评估的代数。

* [常量类型](../syntax/type/advanced/const.md)
* [常量表达式](../syntax/type/advanced/const.md)

### 定义

分配与变量对应的对象。

### 提供的属性

可作为 API 使用的属性。特别是由特征自动实现的属性。

＃＃＃ 申请

将参数传递给函数对象并获取评估结果。

### [装饰器](../syntax/29_decorator.md)

``` python
@deco
f x = ...
```

语法糖，或“装饰”。大致等于`_f x = ...; f = 装饰 _f`。 `deco` 本身只是一个高阶子程序。

### 析构函数

对象被销毁时调用的方法。

### 程序 -> [procedure](../syntax/08_procedure.md)

读取和写入可变状态的子程序。
有时会说程序的执行结果可以根据调用过程的顺序而改变，但如果我们谈论交换性，这是不正确的。
例如，作为函数子类型的运算符通常不可交换。

### [默认参数](../syntax/04_function.md/#default arguments default-parameters)

通过指定形式参数的默认值，可以在调用时省略实际参数的指定的函数。

＃＃＃ 扩张

* 扩展运算符
* 扩展分配

### [特殊格式](../syntax/../API/special.md)

不能作为实际参数传递的对象。

### 匿名函数 -> [anonymous function](../syntax/20_lambda.md)

由匿名函数运算符`->`创建的函数对象。可以在不定义名称的情况下使用。

### 点运算符 (`.`) -> attribute reference

### 顶部

* 顶部类型 -> [结构对象]
* 顶级 -> [对象]

### [特征](../syntax/type/03_trait.md)

## na line

### [理解](../syntax/27_comprehension.md)

### ~~中缀运算符~~ -> 中缀运算符

### 命名空间

## 是一行

### [数组](../syntax/10_array.md)

### [派生类型](../syntax/type/variances.md/# 用户定义的类型变体)

### [模式（匹配）]（../syntax/26_pattern_matching.md）

### [包](../syntax/33_package_system.md)

### hashmap -> [dict](../syntax/11_dict.md)

### [补丁](../syntax/type/07_patch.md)

### 公共变量-> [public variables](../syntax/19_visibility.md)

### 参数 -> [argument](../syntax/04_function.md)

### [参数多态](../syntax/type/overloading.md)

### [逆变](../syntax/type/advanced/variance.md)

### 相比

* 比较运算符
* 可比类型

### [私有变量](../syntax/19_visibility.md)

### 标准

* 标准输出
* 标准输入
* 标准库

### [副作用](../syntax/07_side_effect.md)

代码应该/不应该读/写外部可变状态。

### 复数 -> 复数

### 浮动 -> 浮动

### 私有变量 -> 私有变量

### 布尔代数-> Bool

### [程序](../syntax/08_procedure.md)

### [参数](../syntax/04_function.md)

### 部分类型 -> Subtyping

### [不可变]

在 Erg 中，一个对象永远不应该改变它的内容。

* [不可变对象]
* [不可变类型]
* [不可变引用]

### [筛子类型](../syntax/type/12_refinement.md)

### [堵塞]

### 解构赋值

### [变量](../syntax/02_variable.md)

### 底部

* 底部类型 -> [{}]
* 底层 -> [从不]

### [多态性]

## ma line

### ~~ 前缀运算符 ~~ -> 前缀运算符

### [标记类型](../syntax/type/advanced/marker_trait.md)

### [匿名函数](../syntax/21_lambda.md)

### 可变 -> [可变]

### [移动]

### 方法

### 元字符

### [模块](../syntax/24_module.md)

### [字符串] -> [字符串]

* [字符串插值](../syntax/01_literal.md/#Str 字面量)

### 返回值

## 或行

### [幻像类型](../syntax/type/advanced/phantom.md)

### 请求属性

### [元素]

### [称呼]

## 拉线

### [图书馆]

### lambda 表达式 -> [匿名函数](../syntax/20_lambda.md)

### 排名

* [rank2 多态性](../syntax/type/advanced/rank2type.md)

### [文字](../syntax/01_literal.md)

* [文字标识符](../syntax/18_naming_rule.md/#literal identifier)

### [量化](../syntax/type/quantified.md)

### [布局](../syntax/type/mut.md)

### [枚举](../syntax/type/10_enum.md)

### [记录](../syntax/12_record.md)

* [记录类型]
* 记录多态 -> Column Polymorphism

### 列多态

### [局部变量](../syntax/19_visibility.md)

## 线

### 通配符