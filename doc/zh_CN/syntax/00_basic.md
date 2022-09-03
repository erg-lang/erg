# 基本信息

> ：此文档尚未完成。未进行校样（文体、正确链接等）。此外，Erg 的语法在 0.* 版本之间可能会有颠覆性的改变，随之而来的文档更新可能跟不上。请事先谅解。
> 此外，如果你发现本文档中的错误，请从或<gtr=“11”/>提出更正建议。

本文档介绍了 Erg 的基本语法。和<gtr=“13”/>位于不同的目录中。

## Hello, World!

首先按照惯例举办 Hello World 活动吧。


```erg
print!("Hello, World!")
```

跟 Python 和同系语言差不多。引人注目的是后面的<gtr=“15”/>，我会慢慢解释它的含义。此外，在 Erg 中，如果解释不准确，可以省略括号<gtr=“16”/>。与 Ruby 类似，它可以省略括号，但它不能具有多个解释，也不能在参数为 0 时省略<gtr=“17”/>，就像 Python 一样。


```erg
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OK, but this does not mean to call, simply to get `print!` as a callable object

print! f x # OK, interpreted as `print!(f(x))`
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print!(f x, y) # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print! f(x, g y, z) # NG, can be taken to mean either `print!(x, g(y), z)` or `print!(x, g(y, z))`
```

## 脚本

Erg 代码称为脚本。可以以文件格式（.er）保存和运行脚本。

## 注释

及更高版本将作为注释忽略。当你想要解释代码的意图，或者想要暂时禁用代码时，可以使用此选项。


```erg
# コメント
## `#`以降は改行されるまで無視されるので、`#`は何個あってもOK
#[
複数行コメント
対応する`]#`のところまでずっとコメントとして扱われます
]#
```

## 表达式，分隔符

脚本是一系列表达式（expression）。表达式是一个可以计算和评估的东西，在 Erg 中几乎所有的东西都是表达式。使用分隔符-换行符或分号-分隔每个表达式。Erg 脚本基本上是从左到右、从上到下进行评估的。


```erg
n = 1 # 代入式
f(1, 2) # 関数適用式
1 + 1 # 演算子適用式
f(1, 2); 1 + 1
```

有一个称为即时块的功能，它使用块中最后计算的表达式作为变量的值，如下所示。这与无参数函数不同，它不使用。请注意，方块只在现场评估一次。


```erg
i =
    x = 1
    x + 1
assert i == 2
```

这不能通过分号（）来实现。


```erg
i = (x = 1; x + 1) # SyntaxError: cannot use `;` in parentheses
```

## 缩进

Erg 使用与 Python 相同的缩进来表示块。触发块开始的运算符（特殊格式）有五种：，<gtr=“23”/>，<gtr=“24”/>，<gtr=“25”/>和<gtr=“26”/>（其他运算符不是，但<gtr=“27”/>和<gtr=“28”/>也会生成缩进）。它们各自的含义将在后面介绍。


```erg
f x, y =
    x + y

for! 0..9, i =>
    print! i

for! 0..9, i =>
    print! i; print! i

ans = match x:
    0 -> "zero"
    _: 0..9 -> "1 dight"
    _: 10..99 -> "2 dights"
    _ -> "unknown"
```

如果一行太长，可以使用在中间换行。


```erg
# this does not means `x + y + z` but means `x; +y; +z`
x
+ y
+ z

# this means `x + y + z`
x \
+ y \
+ z
```

<p align='center'>
    Previous | <a href='./01_literal.md'>Next</a>
</p>
