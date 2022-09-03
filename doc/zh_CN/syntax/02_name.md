# 变量

变量是代数的一种。Erg 中的代数-有时也称为变量（如果正确）-是指命名对象并使其可从代码中的其他位置使用的功能。

变量定义如下。部分称为变量名（或标识符），<gtr=“17”/>称为赋值运算符，<gtr=“18”/>部分称为赋值。


```erg
n = 1
```

以这种方式定义的随后可用作表示整数对象<gtr=“20”/>的变量。此系统称为赋值（或绑定）。我们刚才提到了<gtr=“21”/>是一个对象。我们将在后面讨论对象是什么，但我们现在应该将其赋值到赋值运算符（例如<gtr=“22”/>）的右侧。

如果要指定变量的类型。类型是指对象所属的集合，这也将在后面介绍。指定为自然数（<gtr=“24”/>）。


```erg
n: Nat = 1
```

请注意，与其他语言不同，多重赋值是不可能的。


```erg
# NG
l1 = l2 = [1, 2, 3] # SyntaxError: 多重代入はできません
# OK
l1 = [1, 2, 3]
l2 = l1.clone()
```

也不能对变量进行重新赋值。可以使用的功能，即保持可变状态的功能将在后面讨论。


```erg
i = 1
i = i + 1 # AssignError: cannot assign twice
```

你可以在内部范围内定义具有相同名称的变量，但它们只是放在上面，而不是破坏性地重写值。如果返回到外部范围，则值也将返回。请注意，这与 Python“语句”的作用域不同。这类功能通常称为阴影。但是，与其他语言的阴影不同，你不能在同一范围内进行阴影。


```erg
x = 0
# x = 1 # AssignError: cannot assign twice
if x.is_zero(), do:
    x = 1 # 外側のxとは同名の別物
    assert x == 1
assert x == 0
```

以下乍一看似乎可行，但还是不行。这不是技术限制，而是设计判断。


```erg
x = 0
if x.is_zero(), do:
    x = x + 1 # NameError: cannot define variables refer to variables with the same name
    assert x == 1
assert x == 0
```

## 常数

常数也是代数的一种。如果标识符以大写字母开头，则将其视为常量。它被称为常量，因为它一旦定义就不会改变。部分称为常量名称（或标识符）。其他与变量相同。


```erg
N = 0
if True, do:
    N = 1 # AssignError: constants cannot be shadowed
    pass()
```

常量在定义的范围之后变得不变。我也不能阴影。由于该性质，常量可用于模式匹配。后面我们会讨论模式匹配。

你可能希望将常量用于不变的值，如数学常量或有关外部资源的信息。除之外的对象通常是全部大写字母（所有字符都是大写的样式）。


```erg
PI = 3.141592653589793
URL = "https://example.com"
CHOICES = ["a", "b", "c"]
```


```erg
PI = 3.141592653589793
match! x:
    PI => print! "π"
    other => print! "other"
```

当为<gtr=“28”/>时，上面的代码输出<gtr=“29”/>。如果将<gtr=“30”/>更改为其他数字，则输出<gtr=“31”/>。

有些常量是不能赋值的。可变对象等等。可变对象是可以更改其内容的对象，如下所述。这是因为常量只能由常量表达式赋值。我们还将在后面讨论常数表达式。


```erg
X = 1 # OK
X = !1 # TypeError: cannot define Int! object as a constant
```

## 删除代数

可以使用函数删除代数。所有依赖于代数（直接引用代数的值）的其他代数都将被删除。


```erg
x = 1
y = 2
Z = 3
f a = x + a

assert f(2) == 3
Del x
Del y, Z

f(2) # NameError: f is not defined (deleted in line 6)
```

但是，只能删除模块中定义的代数。不能删除内置常量，如<gtr=“34”/>。


```erg
Del True # TypeError: cannot delete built-in constants
Del print! # TypeError: cannot delete built-in variables
```

## Appendix：赋值等价性

注意，当时，不一定是<gtr=“36”/>。例如有<gtr=“37”/>。这是由 IEEE 754 规定的正式浮点数的规格。


```erg
x = Float.NaN
assert x != Float.NaN
assert x != x
```

其他，也存在原本就没有定义等值关系的对象。


```erg
f = x -> x**2 + 2x + 1
g = x -> (x + 1)**2
f == g # TypeError: cannot compare function objects

C = Class {i: Int}
D = Class {i: Int}
C == D # TypeError: cannot compare class objects
```

严格地说，并不是将右边值直接代入左边的识别符。函数对象和类对象的情况下，对对象进行赋予变量名的信息等的“修饰”。但是结构型的情况不受此限制。


```erg
f x = x
print! f # <function f>
g x = x + 1
print! g # <function g>

C = Class {i: Int}
print! C # <class C>
```

<p align='center'>
    <a href='./01_literal.md'>Previous</a> | <a href='./03_declaration.md'>Next</a>
</p>
