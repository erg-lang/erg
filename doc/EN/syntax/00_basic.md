# Basics

> __Warning__: This document is incomplete. It has not been proofread (style, correct links, mistranslation, etc.). Also, Erg's syntax may be change destructively during version 0.*, and the documentation may not have been updated accordingly. Please be aware of this beforehand.
> If you find any errors in this document, please report then to [here form](https://forms.gle/HtLYRfYzWCAaeTGb6) or [GitHub repo](https://github.com/erg-lang/erg/issues/new?assignees=&labels=bug&template=bug_report.yaml). We would appreciate your suggestions.
>
> [Erg Book 日本語訳](https://erg-lang.org/the-erg-book/JA/)
>
> [Erg Book 繁體中文翻譯](https://erg-lang.org/the-erg-book/zh_TW/)
>
> [Erg Book 简体中文翻译](https://erg-lang.org/the-erg-book/zh_CN/)

This document describes the basic syntax of Erg.
If you already have experience with languages such as Python, please refer to the [quick tour](quick_tour.md) for an overview.
There is also a separate [Standard API](../API/index.md) and [Internal Documentation for Erg Contributors](../dev_guide/index.md). If you need a detailed explanation of the syntax or Erg itself, please refer to those documents.

## Hello, World&excl;

First, let's do "Hello World".

```python
print!("Hello, World!")
```

This is almost identical to Python and other languages in the same family. The most striking feature is the `!`, the meaning of which will be explained later.
In Erg, parentheses `()` can be omitted unless there is some confusion in interpretation.
The omission of parentheses is similar to Ruby, but it is not possible to omit parentheses that can be interpreted in more than one way.

```python
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OK, but this does not mean to call, simply to get `print!` as a callable object

print! f x # OK, interpreted as `print!(f(x))`
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))` print!
print!(f x, y) # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print! f(x, g y, z) # NG, can be taken to mean either `print!(x, g(y), z)` or `print!(x, g(y, z))`
```

## Scripts

Erg code is called a script. Scripts can be saved and executed in file format (.er).

## REPL/File Execution

To start REPL, simply type:

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

## Comments

The code after `#` is ignored as a comment. Use this to explain the intent of the code or to temporarily disable the code.

```python
# Comment
# `#` and after are ignored until a new line is inserted
#[
Multi-line comment
Treated as a comment all the way up to the corresponding `]#`
]#
```

## Expressions, separators

A script is a series of expressions. An expression is something that can be calculated or evaluated, and in Erg almost everything is an expression.
Each expression is separated by a separator - either a new line or a semicolon `;`-.
Erg scripts are basically evaluated from left to right, top to bottom.

```python
n = 1 # assignment expression
f(1, 2) # function-call expression
1 + 1 # operator-call expression
f(1, 2); 1 + 1
```

As shown below, there is a syntax called instant block that takes the last expression evaluated in the block as the value of the variable.
This differs from a function with no arguments, which does not add `()`. Note that instant blocks are evaluated only once on the fly.

```python
i =
    x = 1
    x + 1
assert i == 2
```

This cannot be accomplished with a semicolon (`;`).

```python
i = (x = 1; x + 1) # SyntaxError: cannot use `;` in parentheses
```

## Indentation

Erg, like Python, uses indentation to represent blocks. There are five operators (special forms) that trigger the start of a block: `=`, `->`, `=>`, `do`, and `do!` (In addition, `:` and `|`, although not operators, also produce indentation). The meanings of each are described later.

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

If a line is too long, it can be broken using `\`.

```python
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
