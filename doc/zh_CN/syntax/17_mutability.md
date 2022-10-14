# 可变性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/17_mutability.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/17_mutability.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

正如我们已经看到的，所有 Erg 变量都是不可变的。 但是，Erg 对象具有可变性的概念
以下面的代码为例

```python
a = [1, 2, 3]
a = a + [4, 5, 6]
print! a # [1, 2, 3, 4, 5, 6]
```

上面的代码实际上不能被 Erg 执行。 这是因为它不可重新分配

可以执行此代码

```python
b = ![1, 2, 3]
b.concat! [4, 5, 6]
print! b # [1, 2, 3, 4, 5, 6]
```

`a, b` 的最终结果看起来一样，但它们的含义却大不相同
虽然 `a` 是表示 `Nat` 数组的变量，但第一行和第二行指向的对象是不同的。 名称`a`相同，但内容不同

```python
a = [1, 2, 3]
print! id! a # 0x000002A798DFE940
_a = a + [4, 5, 6]
print! id! _a # 0x000002A798DFE980
```

`id!` 过程返回对象驻留的内存地址

`b` 是一个 `Nat` "动态" 数组。 对象的内容发生了变化，但变量指向的是同一个东西

```python
b = ![1, 2, 3]
print! id! b # 0x000002A798DFE220
b.concat! [4, 5, 6]
print! id! b # 0x000002A798DFE220
```

```python
i = !0
if! True. do!
    do! i.inc!() # or i.add!(1)
    do pass
print! i # 1
```

`!` 是一个特殊的运算符，称为 __mutation 运算符__。 它使不可变对象可变
标有"！"的对象的行为可以自定义

```python
Point = Class {.x = Int; .y = Int}

# 在这种情况下 .x 是可变的，而 .y 保持不变
Point! = Class {.x = Int!; .y = Int}
Point!.
    inc_x! ref!(self) = self.x.update! x -> x + 1

p = Point!.new {.x = !0; .y = 0}
p.inc_x!()
print! p.x # 1
```

## 常量

与变量不同，常量在所有范围内都指向同一事物
常量使用 `=` 运算符声明

```python
PI = 3.141592653589
match! x:
    PI => print! "this is pi"
```

常量在全局以下的所有范围内都是相同的，并且不能被覆盖。因此，它们不能被 ``=`` 重新定义。此限制允许它用于模式匹配
`True` 和 `False` 可以用于模式匹配的原因是因为它们是常量
此外，常量总是指向不可变对象。诸如 `Str!` 之类的类型不能是常量
所有内置类型都是常量，因为它们应该在编译时确定。可以生成非常量的类型，但不能用于指定类型，只能像简单记录一样使用。相反，类型是其内容在编译时确定的记录

## 变量、名称、标识符、符号

让我们理清一些与 Erg 中的变量相关的术语

变量是一种为对象赋予名称以便可以重用(或指向该名称)的机制
标识符是指定变量的语法元素
符号是表示名称的语法元素、记号

只有非符号字符是符号，符号不称为符号，尽管它们可以作为运算符的标识符
例如，`x` 是一个标识符和一个符号。 `x.y` 也是一个标识符，但它不是一个符号。 `x` 和 `y` 是符号
即使 `x` 没有绑定到任何对象，`x` 仍然是一个符号和一个标识符，但它不会被称为变量
`x.y` 形式的标识符称为字段访问器
`x[y]` 形式的标识符称为下标访问器

变量和标识符之间的区别在于，如果我们在 Erg 的语法理论意义上谈论变量，则两者实际上是相同的
在 C 中，类型和函数不能分配给变量； int 和 main 是标识符，而不是变量(严格来说可以赋值，但有限制)
然而，在Erg语中，"一切都是对象"。不仅函数和类型，甚至运算符都可以分配给变量

<p align='center'>
    <a href='./16_iterator.md'>上一页</a> | <a href='./18_ownership.md'>下一页</a>
</p>
