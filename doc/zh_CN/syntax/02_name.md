# 变量和常量

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/02_name.md%26commit_hash%3D14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/02_name.md&commit_hash=14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)

## 变量
变量是一种代数； Erg 中的代数 - 如果没有混淆，有时简称为变量 - 指的是命名对象并使它们可从代码的其他地方引用的功能

变量定义如下
`n` 部分称为变量名(或标识符)，`=` 是赋值运算符，`1` 部分是赋值

```python
n = 1
```

以这种方式定义的"n"此后可以用作表示整数对象"1"的变量。该系统称为分配(或绑定)
我们刚刚说过`1`是一个对象。稍后我们将讨论对象是什么，但现在我们假设它是可以赋值的，即在赋值运算符的右侧(`=` 等)

如果要指定变量的"类型"，请执行以下操作。类型大致是一个对象所属的集合，后面会解释
这里我们指定`n`是自然数(`Nat`)类型

```python
n: Nat = 1
```

请注意，与其他语言不同，不允许多次分配

```python
# NG
l1 = l2 = [1, 2, 3] # 语法错误: 不允许多重赋值
# OK
l1 = [1, 2, 3]
l2 = l1.clone()
```

也不能重新分配给变量。稍后将描述可用于保存可变状态的语法

```python,compile_fail
i = 1
i = i + 1 # 分配错误: 不能分配两次
```

您可以在内部范围内定义具有相同名称的变量，但您只是覆盖它，而不是破坏性地重写它的值。如果您返回外部范围，该值也会返回
请注意，这是与 Python "语句"范围不同的行为
这种功能通常称为阴影。但是，与其他语言中的阴影不同，您不能在同一范围内进行阴影

```python
x = 0
# x = 1 # 赋值错误: 不能赋值两次
if x.is_zero(), do:
    x = 1 # 与同名的外部 x 不同
    assert x == 1
assert x == 0
```

乍一看，以下内容似乎可行，但仍然不可能。这是一个设计决定，而不是技术限制

```python
x = 0
if x.is_zero(), do:
    x = x + 1 # 名称错误: 无法定义变量引用同名变量
    assert x == 1
assert x == 0
```

## 常量

常数也是一种代数。如果标识符以大写字母开头，则将其视为常量。它们被称为常量，因为一旦定义，它们就不会改变
`N` 部分称为常量名(或标识符)。否则，它与变量相同

```python
N = 0
if True, do:
    N = 1 # 赋值错误: 常量不能被遮蔽
    pass()
```

常量在定义的范围之外是不可变的。他们不能被遮蔽。由于这个属性，常量可以用于模式匹配。模式匹配在后面解释

例如，常量用于数学常量、有关外部资源的信息和其他不可变值

除了 [types](./type/01_type_system.md) 之外的对象标识符使用全大写(所有字母大写的样式)是常见的做法

```python
PI = 3.141592653589793
URL = "https://example.com"
CHOICES = ["a", "b", "c"]
```

```python
PI = 3.141592653589793
match! x:
    PI => print! "π"
    other => print! "other"
```

当 `x` 为 `3.141592653589793` 时，上面的代码会打印 `π`。如果 `x` 更改为任何其他数字，它会打印 `other`

有些对象不能绑定为常量。例如，可变对象。可变对象是其状态可以改变的对象，后面会详细介绍
这是因为只有常量表达式才能分配给常量的规则。常量表达式也将在后面讨论

```python
X = 1 # OK
X = !1 # 类型错误: 无法定义 Int！ 对象作为常量
```

## 删除变量

您可以使用 `Del` 函数删除变量。依赖于变量的所有其他变量(即直接引用变量值的变量)也将被删除

```python
x = 1
y = 2
z = 3
f a = x + a

assert f(2) == 3
Del x
Del y, z

f(2) # 名称错误: f 未定义(在第 6 行中删除)
```

注意 `Del` 只能删除用户自定义模块中定义的变量。无法删除诸如"True"之类的内置常量

```python
Del True # 类型错误: 无法删除内置常量
Del print! # TypeError: 无法删除内置变量
```

## 附录: 赋值和等价

请注意，当 `x = a` 时，`x == a` 不一定为真。一个例子是`Float.NaN`。这是 IEEE 754 定义的浮点数的正式规范

```python
x = Float.NaN
assert x ! = NaN
assert x ! = x
```

还有其他对象首先没有定义等价关系

```python,compile_fail
f = x -> x**2 + 2x + 1
g = x -> (x + 1)**2
f == g # 类型错误: 无法比较函数对象

C = Class {i: Int}
D = Class {i: Int}
C == D # 类型错误: 无法比较类对象
```

严格来说，`=` 不会将右侧的值直接分配给左侧的标识符
在函数和类对象的情况下，执行"修改"，例如将变量名称信息赋予对象。但是，结构类型并非如此

```python
f x = x
print! f # <函数 f>
g x = x + 1
print! g # <函数 g>

C = Class {i: Int}
print! C # <类 C>
```

<p align='center'>
    <a href='./01_literal.md'>上一页</a> | <a href='./03_declaration.md'>下一页</a>
</p>
