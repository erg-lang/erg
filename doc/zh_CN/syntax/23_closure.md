# 封闭

Erg 子例程具有一个名为“闭包”的功能，用于捕获外部变量。


```erg
outer = 1
f x = outer + x
assert f(1) == 2
```

可以捕捉可变对象，也可以捕捉不变对象。


```erg
sum = !0
for! 1..10, i =>
    sum.add! i
assert sum == 45

p! x =
    sum.add! x
p!(1)
assert sum == 46
```

但需要注意的是，函数无法捕获可变对象。如果可以在函数中引用可变对象，则可以编写如下所示的代码。


```erg
# !!! 这个代码实际上给出了一个错误!!!
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

函数应该为相同的参数返回相同的值，但假设已被破坏。请注意，是在调用时首次计算的。

如果需要函数定义时可变对象的内容，则调用。


```erg
i = !0
immut_i = i.clone().freeze()
f x = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## 避免可变状态，函数编程


```erg
# Erg
sum = !0
for! 1..10, i =>
    sum.add! i
assert sum == 45
```

在 Python 中，可以按如下方式编写上面的等效程序。


```python
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

但 Erg 建议使用更简单的写法。使用局部化使用函数的状态的样式，而不是使用子例程和可变对象来维护状态。这称为函数型编程。


```erg
# Functional style
sum = (1..10).sum()
assert sum == 45
```

上面的代码与刚才的结果完全相同，但我们可以看到它要简单得多。

除了求和之外，还可以使用函数执行更多操作。<gtr=“12”/>是迭代器方法，它为每个小版本执行参数<gtr=“13”/>。存储结果的计数器的初始值由<gtr=“14”/>指定，然后存储在<gtr=“15”/>中。


```erg
# start with 0, result will
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

Erg 的设计是为了使用不变的对象进行编程，从而提供自然简洁的描述。

<p align='center'>
    <a href='./22_subroutine.md'>Previous</a> | <a href='./24_module.md'>Next</a>
</p>
