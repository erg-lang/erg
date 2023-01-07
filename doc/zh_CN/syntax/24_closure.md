# 闭包

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_closure.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_closure.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

Erg子例程有一个称为"闭包"的功能，可以捕获外部变量

```python
outer = 1
f x = outer + x
assert f(1) == 2
```

与不可变对象一样，可变对象也可以被捕获

```python
sum = !0
for! 1..10, i =>
    sum.add!i
assert sum == 45

p!x=
    sum.add!x
p!(1)
assert sum == 46
```

但是请注意，函数不能捕获可变对象
如果可以在函数中引用可变对象，则可以编写如下代码

```python,compile_fail
# !!! 这段代码实际上给出了一个错误！！！
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

该函数应该为相同的参数返回相同的值，但假设被打破了
请注意，`i`仅在调用时进行评估

如果您想在定义函数时获取可变对象的内容，请调用`.clone`

```python
i = !0
immut_i = i.clone().freeze()
fx = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## avoid mutable state, functional programming

```python
# Erg
sum = !0
for! 1..10, i =>
    sum.add!i
assert sum == 45
```

上面的等效程序可以用Python编写如下:

```python,checker_ignore
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

但是，Erg 建议使用更简单的表示法
与其使用子例程和可变对象来传递状态，不如使用一种使用函数来定位状态的风格。这称为函数式编程

```python
# 功能风格
sum = (1..10).sum()
assert sum == 45
```

上面的代码给出了与之前完全相同的结果，但是您可以看到这个代码要简单得多

`fold`函数可以用来做比sum更多的事情
`fold`是一个迭代器方法，它为每次迭代执行参数`f`
累加结果的计数器的初始值在`init`中指定，并在`acc`中累加

```python
# 从0开始，结果会
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

Erg被设计为对使用不可变对象进行编程的自然简洁描述

<p align='center'>
    <a href='./23_subroutine.md'>上一页</a> | <a href='./25_module.md'>下一页</a>
</p>