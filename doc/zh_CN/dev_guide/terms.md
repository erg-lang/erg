# 术语词典

## 符号

### &excl;

过程或附加在标识符末尾的标记，以指示其为可变类型。或者变量运算符。

### ../syntax/00_basic.md/# 注释

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

### 断言

检查代码中的条件是否成立（通常是在运行时）。使用函数等进行操作。


```erg
sum = !0
for! 0..10, i =>
    sum.add! i

assert sum == 55
```

### 值对象

在 Erg 中，与基本对象相同。编译时可以进行评价，拥有不言而喻的比较方法。

### 附着面片../syntax/29_decorate.md#attach

为 Tracet 提供标准实现的补丁程序。

### 即席多相->

所谓超载的多相。

### 属性-属性

标识符中的<gtr=“9”/>部分。

### 安利

运算符使用多少个操作数。

### 依赖关系../syntax/type/dependent_type.md

以值（通常为非类型）为参数的类型。

### 可变体-> 不可变

表示目标保持不变。在其他语言中，变量也具有可变/可变特性，但在 Erg 中，变量都是可变的。

### 参数-> 参数

### 实例

类创建的对象。类类型的元素。

### 即时块（../syntax/00_basic.md# 表达式分隔符）


```erg
x =
    y = f(a)
    z = g(b, c)
    y + z
```

### 索引

形式为，或其中<gtr=“11”/>的部分。称为 Indexable 对象。

### 缩进../syntax/00_basic.md# 缩进

靠空格使句子向右靠。缩进。Erg 通过缩进来表现块。这叫做越位规则。

### 别名

别名。

### 错误

规范规定的异常状态。

* [エラーハンドリング]

### 运算符../syntax/06_operator.md

将运算应用于操作数的对象。或表示对象的符号。

* [演算子の結合強度]

### 覆盖

用子类覆盖超类的方法。在 Erg 中，覆盖时必须安装装饰器。

### 禁止过载（../syntax/type/overloading.md）

### 越位规则->

### 对象

* 面向对象

### 操作数->

### 操作员->

## 家行

### 卡印（../syntax/type/advanced/kind.md）

所谓模子的模子。

### 可见性

标识符是否可从外部（范围外或单独模块、单独软件包）引用的性质。

### 类型

要对项进行分组的对象。

* [型指定]
* 清除类型（../syntax/type/advanced/erasure.md）
* [型推論]
* 类型注释../syntax/type/conv_type.md
* [型引数]
* 添加类型（../syntax/type/advanced/erasure.md）
* 类型变量（../syntax/type/type_variable.md）
* [型制約]

### 保护

### 封装

隐藏实现细节。

### 变量

不可变。

* [可変オブジェクト]
* [可変型]
* [可変参照]
* [可変配列]
* [可変長引数]

### 函数../syntax/04_function.md

没有副作用的子程序。

* 函数型编程（../syntax/23_scop.md# 避免变量状态函数型编程）

### 基本类型

### 记名的

通过名称而不是对称结构来区分。

* [记名型]->
* [記名化]
* 记名部分类型../syntax/type/05_nst_vs_sst.md

### 捕捉-> 闭包

### 协变

在 Erg 中，当时，如果<gtr=“19”/>，则<gtr=“20”/>为协变。

### 关键字参数

函数调用形式中的<gtr=“22”/>。实际自变量可以用假自变量名而不是顺序指定。

### 空集->[{}]

### 区间

* 间隔类型（../syntax/type/11_interval.md）
* 区间运算符

### 嵌入

未在.er 文件中实现的 Erg 标准 API。

### 类../syntax/type/04_class.md

具有继承功能的结构和抽象数据类型。在 Erg 中是为了实现记名式分型以及覆盖的类型。在其他语言中也有承担模块和型的责任和义务的情况，在 Erg 中，模块是模块对象，型是型对象承担其责任和义务。

### 闭合

### 全局变量

### 克隆

### 继承

定义以某个类为上级集合的类。继承源的类称为超类，继承目标的类称为子类。子类具有超类的所有功能。

### 高阶

* 高阶../syntax/type/advanced/kind.md
* 高阶型
* 高阶函数

### 公共变量

### 结构子类型

### ~~ 向后参照 ~~~->[向前参照]

### 复制

### 注释

### 集合../syntax/10_array.md

### 冒号->[：]

### 构造函数（../syntax/type/04_class.md）

### 集装箱

### 编译器

### 编译时计算../syntax/04_function.md# 编译时函数

### 逗号->[，]

## 差行

### 递归

指自己。

* 递归型
* 递归函数../syntax/04_function.md# 递归函数

### 下标-> 索引

### 多相子类型（../syntax/type/overloading.md）

多相分型。子类型是指在类型中与集合的包含关系相对应的类型。

### 子程序

模块化处理的对象。Erg 中函数、过程和方法的通用名称。

### 参考（../syntax/18_memory_management.md# 借用）

* 引用对象
* 参照计数 (RC) （../syntax/18_memory_management.md# 内存管理）
* 参考等效性->

### 标识符（../syntax/02_variable.md/# 赋值）

### 签名

* 类型签名

### 词典../syntax/11_dict.md

### 自然数->Nat

### 通用->[全称类型]

### 发电机

### 投影类型

### 借用->

### 阴影（../syntax/02_name.md# 变量）

在内部作用域中定义一个同名的变量，并覆盖该变量的引用。

### 种子->

大致是个模子。

### 集-> 集

在 Erg 中是 Set 对象。

### 谓语

* [述語関数]

返回布尔类型的函数。

### 条件分歧

### 所有权

关于对象唯一性的概念。如果拥有对象的所有权，则可以对对象进行可变引用。

### 真伪类型-> 布尔

### 单吨

从只能生成一个实例的类生成的实例。也指确保只生成一个类实例的设计模式。

### 符号->

* [シンボル化]

### 脚本../syntax/00_basic.md# 脚本

描述 Erg 程序的文件。

### 范围

变量管理中的单位。外侧的范围不能参照存在于内侧范围的变量。另外，脱离范围时，参照点数为 0 的对象被释放。

### 跨页运算符-> 展开赋值

### 切片../syntax/10_array.md# 切片

以形式生成的表示数组子串的对象。

### 控制字符

### 整数-> 输入

自然数加负数的集合。

### 集../syntax/12_set.md

### 分号->[；]

### 声明../syntax/03_declaration.md

显式设置变量类型。

### 全称

* 全称类型->
  * 封闭全称类型
  * 打开的全称类型
* 全称函数-> 多相关数
* 全称量化

### 前缀运算符

以格式应用的运算符<gtr=“30”/>。

### 互相的递归

### 下标-> 索引

### 属性

* [属性的部分型]

## 多行

### 代数../syntax/02_name.md

* 代数类型（../syntax/type/13_algebraic.md）
* 代数数据类型

### 赋值../syntax/02_variable.md/# 赋值

### 多重

* 多重继承（../syntax/type/07_inheritance.md/# 禁止多重继承）
* 多重赋值
* 多重定义-> 禁止过载

### 多相

* 多相类型（../syntax/type/quantified.md）
* 多相关数

### 多态-> 多态

### 烤鸭打字

### 元组（../syntax/11_tuple.md）

### 单相

* 单相化
* 单相型
* 单相关数

### 延迟初始化

### 抽出赋值

### 抽象语法树->[AST]

### 中置运算符

以格式应用的运算符。

### 常量../syntax/02_name.md/# 常量

可执行的、编译时可评估的代数。

* 常量类型（../syntax/type/advanced/const.md）
* 常量表达式（../syntax/type/advanced/const.md）

### 定义

分配与变量对应的对象。

### 授课属性

可用作 API 的属性。特别是由trait自动实现的属性。

### 应用

将参数传递给函数对象以获得评估结果。

### 装饰器../syntax/29_decorate.md


```erg
@deco
f x = ...
```

的语法糖，或者。大约等于。本身只是一个高阶子程序。

### 析构

销毁对象时调用的方法。

### 过程->

读取和写入可变状态的子程序。有时会解释程序根据调用顺序的不同，程序的执行结果也会发生变化，但如果说的是可换性的话，这是错误的。例如，作为函数子类型的运算符一般不是可换的。

### 缺省参数../syntax/04_function.md/# 缺省参数 default-parameters

通过为虚拟自变量指定缺省值，调用时可以省略实际自变量指定的功能。

### 展开

* [展開演算子]
* [展開代入]

### 特殊格式（../syntax/../API/special.md）

不能传递给实际参数的对象。

### 匿名函数->

由未命名函数运算符生成的函数对象。不用定义名字就能使用。

### 点运算符（）->[属性引用]

### 顶部

* 顶部类型-> 结构对象
* 顶级-> 对象

### TRAIT（../syntax/type/03_trait.md）

## 标题

### 内涵符号../syntax/27_comprehension.md

### 中置算子 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### 名称空间

## 派系

### 阵列../syntax/10_array.md

### 派生类型（../syntax/type/variances.md/# 用户定义类型的退化）

### 图案（匹配）../syntax/26_pattern_matching.md

### 软件包../syntax/33_package_system.md

### 哈希映射->

### 面片../syntax/type/07_patch.md

### 公共变量->

### 参数->

### 参数化多相（../syntax/type/overloading.md）

### 反变（../syntax/type/advanced/variance.md）

### 比较

* [比較演算子]
* [比較可能型]

### 私有变量../syntax/19_visibility.md

### 标准

* 标准输出
* 标准输入
* 标准库

### 副作用../syntax/07_side_effect.md

代码不能读取或写入外部可变状态。

### 复数->

### 浮点数-> 浮点

### 专用变量-> 专用变量

### 布尔代数-> 布尔

### 程序../syntax/08_procedure.md

### 参数（../syntax/04_function.md）

### 部分类型-> 子类型

### 不变

在 Erg 中，对象不改变其内容。

* [不変オブジェクト]
* [不変型]
* [不変参照]

### 筛型（../syntax/type/12_refinement.md）

### 块

### 分解赋值

### 变量../syntax/02_variable.md

### 底部

* 底部->[{}]
* 底部类->Never

### 多态

## 真行

### 前缀运算符 ~~~~~~ 前缀运算符

### 标记类型../syntax/type/advanced/marker_trait.md

### 无名函数../syntax/21_lambda.md

### 可变-> 可变

### 移动

### 方法

### 元字符

### 模块（../syntax/24_module.md）

### 字符串->Str

* 字符串插值（../syntax/01_literal.md/#Str 文字）

### 返回值

## 夜行

### 幽灵类型（../syntax/type/advanced/phantom.md）

### 请求属性

### 元素

### 调用

## 罗列

### 库

### 拉姆达公式->

### 等级

* 通道 2 多相../syntax/type/advanced/rank2type.md

### 文字（../syntax/01_literal.md）

* 文字标识符（../syntax/18_naming_rule.md/# 文字标识符）

### 量化（../syntax/type/quantified.md）

### 布局（../syntax/type/mut.md）

### 枚举类型（../syntax/type/10_enum.md）

### 记录../syntax/12_record.md

* [レコード型]
* 记录多相-> 列多相

### 列多相

### 局部变量../syntax/19_visibility.md

## 和行

### 通配符
