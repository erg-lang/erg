# 基本

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/00_basic.md%26commit_hash%3D736dcb272d2132883ec7b883f7694829398be61e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/00_basic.md&commit_hash=736dcb272d2132883ec7b883f7694829398be61e)

> __Warning__: 本文档不完整。它未经校对(样式、正确链接、误译等)。此外，Erg 的语法可能在版本 0.* 期间发生破坏性更改，并且文档可能没有相应更新。请事先了解这一点
> 如果您在本文档中发现任何错误，请报告至 [此处的表单](https://forms.gle/HtLYRfYzWCAaeTGb6) 或 [GitHub repo](https://github.com/mtshiba/TheErgBook/issues/new )。我们将不胜感激您的建议

本文档描述 Erg 的基本语法
如果您已经有使用 Python 等语言的经验，请参阅 [快速浏览](quick_tour.md) 了解概览
还有一个单独的 [标准 API](../API/index.md) 和 [Erg 贡献者的内部文档](../dev_guide/index.md)。如果您需要语法或 Erg 本身的详细说明, 请参阅那些文档

## 你好，世界&excl;

首先，让我们做"Hello World"

```python
print!("Hello, World!")
```

这与 Python 和同一家族中的其他语言几乎相同。最显着的Trait是`!`，后面会解释它的含义
在 Erg 中，括号 `()` 可以省略，除非在解释上有一些混淆
括号的省略与 Ruby 类似，但不能省略可以以多种方式解释的括号

```python
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OK, 但这并不意味着调用，只是将 `print!` 作为可调用对象

print! f x # OK, 解释为 `print!(f(x))`
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, 可以理解为 `print!(f(x), y)` 或 `print!(f(x, y))` print!
print!(f x, y) # NG, 可以表示"print！(f(x)，y)"或"print！(f(x，y))"
print! f(x, g y, z) # NG, 可以表示"print！(x，g(y)，z)"或"print！(x，g(y，z))"
```

## 脚本

Erg 代码称为脚本。脚本可以以文件格式 (.er) 保存和执行

## REPL/文件执行

要启动 REPL，只需键入: 

```sh
> erg
```

`>` mark is a prompt, just type `erg`.
Then the REPL should start.

```sh
> erg
Starting the REPL server...
Connecting to the REPL server...
Erg interpreter 0.2.4 (tags/?:, 2022/08/17  0:55:12.95) on x86_64/windows
>>>
```

Or you can compile from a file.

```sh
> 'print! "hello, world!"' >> hello.er

> erg hello.er
hello, world!
```

## 注释

`#` 之后的代码作为注释被忽略。使用它来解释代码的意图或暂时禁用代码

```python
# Comment
# `#` and after are ignored until a new line is inserted
# [
Multi-line comment
Treated as a comment all the way up to the corresponding `]#`
]# 
```

## 表达式，分隔符

脚本是一系列表达式。表达式是可以计算或评估的东西，在 Erg 中几乎所有东西都是表达式
每个表达式由分隔符分隔 - 新行或分号 `;`-
Erg 脚本基本上是从左到右、从上到下进行评估的

```python
n = 1 # 赋值表达式
f(1, 2) # 函数调用表达式
1 + 1 # 运算符调用表达式
f(1, 2); 1 + 1
```

如下所示，有一种称为 Instant block 的语法，它将块中评估的最后一个表达式作为变量的值
这与没有参数的函数不同，它不添加 `()`。请注意，即时块仅在运行中评估一次

```python
i =
    x = 1
    x + 1
assert i == 2
```

这不能用分号 (`;`) 完成

```python
i = (x = 1; x + 1) # 语法错误: 不能在括号中使用 `;`
```

## 缩进

Erg 和 Python 一样，使用缩进来表示块。有五个运算符(特殊形式)触发块的开始: `=`、`->`、`=>`、`do` 和 `do!`(此外，`:` 和 `|` ，虽然不是运算符，但也会产生缩进)。每个的含义将在后面描述

```python
f x, y =
    x + y

for! 0..9, i =>
    print!

for! 0..9, i =>
    print! i; print! i

ans = match x:
    0 -> "zero"
    _: 0..9 -> "1 dight"
    _: 10..99 -> "2 dights"
    _ -> "unknown"
```

如果一行太长，可以使用 `\` 将其断开

```python
# 这不是表示 `x + y + z` 而是表示 `x; +y; +z`
X
+ y
+ z

# 这意味着`x + y + z`
x \
+ y \
+ z
```

<p align='center'>
    上一页 | <a href='./01_literal.md'>下一页</a>
</p>
