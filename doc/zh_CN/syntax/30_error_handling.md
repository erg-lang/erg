# 错误处理系统

主要使用 Result 类型。在 Erg 中，如果丢弃 Error 类型的对象（不在顶层），则会发生错误。

## 异常，与 Python 的互操作

Erg 没有异常机制（Exception）。导入 Python 函数时

* 返回类型
* 类型（可能导致运行时错误）

的两个选项，在中默认为后者。如果要作为前者导入，请在<gtr=“9”/>的<gtr=“10”/>中指定<gtr=“11”/>（<gtr=“12”/>）。

## 异常和结果类型

类型表示可能出现错误的值。使用<gtr=“14”/>处理错误在某些方面优于异常机制。首先，从类型定义可以看出子程序可能会出错，在实际使用时也一目了然。


```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

在上面的示例中，仅此代码并不知道异常是从哪个函数调度的。即使追溯到函数定义，也很难确定该函数是否会出现异常。


```erg
# Erg
try!:
    do!:
        x = foo!()?.bar()
        y = baz!()
        qux!()?
    e =>
        print! e
```

相反，在本示例中，和<gtr=“16”/>可以生成错误。确切地说，<gtr=“17”/>也可能是<gtr=“18”/>类型，但在使用中值时，你必须执行此操作。

使用类型的好处远不止这些。类型也是线程安全的。这意味着错误信息可以在并行执行期间（很容易）传递。

## Context

/<gtr=“22”/>类型不会产生副作用，因此它不具有与异常不同的诸如发送位置之类的信息（上下文），但可以使用<gtr=“23”/>方法将信息添加到<gtr=“24”/>对象。<gtr=“25”/>方法是使用<gtr=“26”/>对象本身来创建新的<gtr=“27”/>对象的方法。它是可链接的，可以有多个上下文。


```erg
f() =
    todo() \
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

注意，属性（如<gtr=“28”/>和<gtr=“29”/>）不是次要属性，因此不是 context，不能覆盖最初生成的属性。

## 栈跟踪

类型由于其方便性，在其他语言中也被广泛采用，但与异常机制相比，其缺点是错误的来源变得更难理解。因此，在 Erg 中，使<gtr=“32”/>对象具有<gtr=“33”/>属性，模拟地再现了异常机制那样的栈跟踪。<gtr=“34”/>是调用对象的数组。每当 Error 对象<gtr=“35”/>（包括<gtr=“36”/>所致）时，它的调用子例程将加载到<gtr=“37”/>中。如果<gtr=“38”/>在环境中<gtr=“39”/>或<gtr=“40”/>，它将死机并显示回溯。


```erg
f x =
    ...
    y = foo.try_some(x)?
    ...

g x =
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
#    ...
#    Foo.try_some, line 10, file "foo.er"
#    10 | y = foo.try_some(x)?
#    module::f, line 23, file "foo.er"
#    23 | y = f(x)?
#    module::g, line 40, file "foo.er"
#    40 | i = g(1)?
# Error: ...
```

## 恐慌

Erg 还存在一个名为的机制来处理不可恢复的错误。不可恢复的错误可能是由外部因素引起的错误，例如软/硬件故障，致命到无法继续执行代码的程度，或者是程序编写者不想要的错误。如果发生这种情况，由于程序员的努力无法使其恢复正常系统，因此当场终止程序。这叫做“恐慌”。

使用函数执行死机。


```erg
panic "something went wrong!"
```

<p align='center'>
    <a href='./29_decorator.md'>Previous</a> | <a href='./31_pipeline.md'>Next</a>
</p>
