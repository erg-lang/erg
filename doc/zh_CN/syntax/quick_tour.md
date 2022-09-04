# Quick Tour

下面的文档旨在让初学者也能理解。对于已经掌握 Python，Rust，Haskell 等语言的人来说，这可能有点多余。

因此，下面将概述性地介绍 Erg 的语法。没有特别提到的部分可以认为和 Python 一样。

## 变量，常量

变量定义为。与 Haskell 一样，一旦定义变量，就无法重写。但是，你可以在另一个范围内进行阴影。


```erg
i = 0
if True:
    i = 1
assert i == 0
```

以大写字母开头的是常量。只有编译时可以计算的内容才可以是常量。此外，常量在定义后的所有作用域中都是相同的。


```erg
PI = 3.141592653589793
match random.random!(0..10):
    PI:
        log "You get PI, it's a miracle!"
```

## 声明

与 Python 不同，你只能先声明变量类型。当然，声明类型必须与实际赋值的对象类型兼容。


```erg
i: Int
i = 10
```

## 函数

你可以像 Haskell 一样定义它。


```erg
fib 0 = 0
fib 1 = 1
fib n = fib(n - 1) + fib(n - 2)
```

可以按如下方式定义未命名函数。


```erg
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## 运算符

Erg 自己的运算符如下所示。

### 变量运算符（！）

就像 Ocaml 的。


```erg
i = !0
i.update! x -> x + 1
assert i == 1
```

## 过程

有副作用的子程序称为过程，并带有。


```erg
print! 1 # 1
```

## 类属函数（多相关数）


```erg
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## 记录

你可以使用 ML 语言中的记录（或 JS 中的对象文字）。


```erg
p = {x = 1; y = 2}
```

## 所有权

Erg 拥有可变对象（使用运算符可变的对象）的所有权，不能从多个位置重写。


```erg
i = !0
j = i
assert j == 0
i # MoveError
```

相反，你可以从多个位置引用不变对象。

## 可见性

如果在变量的前面加上，则该变量将成为公共变量，并且可以被外部模块引用。


```erg
# foo.er
.x = 1
y = 1
```


```erg
foo = import "foo"
assert foo.x == 1
foo.y # VisibilityError
```

## 模式匹配

### 变量模式


```erg
# basic assignment
i = 1
# with type
i: Int = 1
# function
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### 文字模式


```erg
# if `i` cannot be determined to be 1 at compile time, TypeError occurs.
# short hand of `_: {1} = i`
1 = i
# simple pattern matching
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# fibonacci function
fib 0 = 0
fib 1 = 1
fib n: Nat = fib n-1 + fib n-2
```

### 常数模式


```erg
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### 销毁（通配符）模式


```erg
_ = 1
_: Int = 1
right(_, r) = r
```

### 可变长度模式

与后述的元组/数组/记录模式组合使用。


```erg
[i, ...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### 元组图案


```erg
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# ネストしていないなら()を省略可能(1, 2は(1, 2)として扱われる)
m, n = 1, 2
```

### 数组模式


```erg
length [] = 0
length [_, ...rest] = 1 + length rest
```

#### 记录模式


```erg
{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### 数据类模式


```erg
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## 内涵记载


```erg
odds = [i | i <- 1..100; i % 2 == 0]
```

## 类

Erg 不支持多级和多级继承。

## trait

与 Rust 的trait类似，但更接近原意，可以合成和分离，属性和方法是对等的。也不涉及实施。


```erg
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## 补丁

你可以为类和trait提供实现。

## 筛子型

可以在谓词表达式中限制类型。


```erg
Nat = {I: Int | I >= 0}
```

## 包含值的参数化（从属）


```erg
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```
