# Erg Compiler Errors

## AssignError

尝试重写不可变变量时发生

## AttributeError

尝试访问不存在的属性时发生

## PurityError

当您在不允许副作用的范围内（函数、不可变类型等）编写导致副作用的代码时发生

## MoveError

尝试访问已移动的变量时发生

## BorrowError

在存在对对象的借用时尝试获取可变引用时发生

## CyclicError

当你有一个明显不可阻挡的循环时发生

```python
i: Int = i

f(): Int = g()
g() = f()

h(): Int = module::h()

T = U
U = T
```

## BytecodeError

当加载的字节码损坏时发生

## CompileSystemError

在编译器内部发生错误时发生

## EnvironmentError

如果您在安装期间没有访问权限，则会发生这种情况

## FeatureError

在检测到未正式提供的实验性功能时发生

## ImportError

## IndentationError

检测到不良缩进时发生
派生自SyntaxError

## NameError

当您访问不存在的变量时发生

## NotImplementedError

当您调用具有定义但没有实现的 API 时发生
派生自 TypeError

## PatternError

当检测到非法模式时发生
派生自SyntaxError

## SyntaxError

在检测到错误语法时发生

## TabError

在使用制表符进行缩进/间距时发生
派生自SyntaxError

## TypeError

当对象类型不匹配时发生

## UnboundLocalError

在定义之前使用变量时发生
更准确地说，它发生在以前使用过在范围内定义的变量时

```python
i = 0
f x =
    y = i + x
    i = 1
    y + i
```

在这段代码中，`y = i + x` 中的 `i` 是一个未定义的变量
但是，常量可以在定义之前在另一个函数中调用

```python
f() = g()
g() = f()
```

## Erg Compiler Warnings

## SyntaxWarning

它在语法上很好，但是当我们检测到冗余或不常见的代码（不必要的 `()` 等）时就会发生这种情况

```python
if (True): # SyntaxWarning: unnecessary parentheses
    ...
```

## DeprecationWarning

在不推荐使用引用的对象时发生
（开发人员在生成此警告时应始终提供替代方法作为提示）

## FutureWarning

当您检测到将来可能导致问题的代码时发生
此警告是由版本兼容性问题（包括库）以及语法和 API 的更改引起的

## ImportWarning
