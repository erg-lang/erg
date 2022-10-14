# 快速浏览

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/quick_tour.md%26commit_hash%3D51016775279ecd28b87178dab6a493a645abb171)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/quick_tour.md&commit_hash=51016775279ecd28b87178dab6a493a645abb171)

`syntax` 下面的文档是为了让编程初学者也能理解而编写的
对于已经掌握 Python、Rust、Haskell 等语言的人来说，可能有点啰嗦

所以，这里是 Erg 语法的概述
请认为未提及的部分与 Python 相同

## 基本计算

Erg 有一个严格的类型。但是, 由于类和特征提供的灵活性, 类型会自动转换为子类型(有关详细信息，请参阅 [API])

另外，不同的类型可以相互计算，只要类型是数值类型即可

```python
a = 1 # 1: Nat
b = a - 10 # -9: Int
c = b / 2 # -4.5: Float
d = c * 0 # -0.0: Float
e = f // 2 # 0: Nat
```

如果您不想允许这些隐式类型转换，您可以在声明时指定类型以在编译时将它们检测为错误

```python
a = 1
b: Int = a / 2
# 错误信息
Error[#0047]: File <stdin>, line 1, in <module>
2│ b: Int = int / 2
   ^
类型错误: ratio的类型不匹配: 
期待:  Int
但找到: Float
```
## 布尔类型
`True` 和 `False` 是 Boolean 类型的单例，但它们也可以转换为 Int 类型
因此，如果它们是 Int 类型，则可以进行比较，但与其他类型比较会导致错误

```python
True == 1 # OK
False == 0 # OK
True == 1.0 # NG
False == 0.0 # NG
True == "a" # NG
```

## 变量，常量

变量用 `=` 定义。 与 Haskell 一样，变量一旦定义就不能更改。 但是，它可以在另一个范围内被遮蔽

```python
i = 0
if True:
    i = 1
assert i == 0
```

任何以大写字母开头的都是常数。 只有可以在编译时计算的东西才能是常量
此外，自定义以来，常量在所有范围内都是相同的

```python
PI = 3.141592653589793
match random.random!(0..10):
    PI ->
        log "You get PI, it's a miracle!"
```

## 类型声明

与 Python 不同的是，只能先声明变量类型
当然，声明的类型和实际分配的对象的类型必须兼容

```python
i: Int
i = 10
```

## 函数

你可以像在 Haskell 中一样定义它

```python
fib0 = 0
fib1 = 1
fibn = fib(n - 1) + fib(n - 2)
```

匿名函数可以这样定义: 

```python
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## 运算符

特定于 Erg 的运算符是: 

### 变异运算符 (!)

这就像 Ocaml 中的`ref`

```python
i = !0
i.update! x -> x + 1
assert i == 1
```

## 程序

具有副作用的子例程称为过程，并标有`!`

您不能在函数中调用过程

```python
print! 1 # 1
```

## 泛型函数(多相关)

```python
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## 记录

您可以使用类似 ML 的语言中的记录等价物(或 JS 中的对象字面量)

```python
p = {x = 1; y = 2}
assert p.x == 1
```

## 所有权

Ergs 由可变对象(使用 `!` 运算符突变的对象)拥有，并且不能从多个位置重写

```python
i = !0
j = i
assert j == 0
i#移动错误
```

另一方面，不可变对象可以从多个位置引用

## 可见性

使用 `.` 前缀变量使其成为公共变量并允许从外部模块引用它

```python
# foo.er
.x = 1
y = 1
```

```python
foo = import "foo"
assert foo.x == 1
foo.y # 可见性错误
```

## 模式匹配

### 变量模式

```python
# 基本任务
i = 1
# with 类型
i: Int = 1
# 函数
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### 文字模式

```python
# 如果 `i` 在编译时无法确定为 1，则发生 类型错误
# 简写: `_ {1} = i`
1 = i
# 简单的模式匹配
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# 斐波那契函数
fib0 = 0
fib1 = 1
fibn: Nat = fibn-1 + fibn-2
```

### 常量模式

```python
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### 丢弃(通配符)模式

```python
_ = 1
_: Int = 1
right(_, r) = r
```

### 可变长度模式

与稍后描述的元组/数组/记录模式结合使用

```python
[i,...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### 元组模式

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# 如果不嵌套，() 可以省略(1, 2 被视为(1, 2))
m, n = 1, 2
```

### 数组模式

```python
length[] = 0
length[_, ...rest] = 1 + lengthrest
```

#### 记录模式

```python
{sin; cos; tan; ...} = import "math"
{*} = import "math" # 全部导入

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### 数据类模式

```python
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## 理解(Comprehensions)

```python
odds = [i | i <- 1..100; i % 2 == 0]
```

## Class

Erg 不支持多重继承

## 特质

它们类似于 Rust 特征，但在更字面意义上，允许组合和解耦，并将属性和方法视为平等
此外，它不涉及实施

```python
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## 修补

您可以为类和特征提供实现

## 筛子类型

谓词表达式可以是类型限制的

```python
Nat = {I: Int | I >= 0}
```

## 带值的参数类型(依赖类型)

```python
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```
