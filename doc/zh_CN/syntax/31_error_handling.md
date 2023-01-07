# 错误处理系统

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/30_error_handling.md%26commit_hash%3Dfba8b193ce4270cb8c9236c4ed7bb8b2497af3fd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/30_error_handling.md&commit_hash=fba8b193ce4270cb8c9236c4ed7bb8b2497af3fd)

主要使用Result类型
在Erg中，如果您丢弃Error类型的对象(顶层不支持)，则会发生错误

## 异常，与 Python 互操作

Erg没有异常机制(Exception)。导入Python函数时

* 将返回值设置为`T 或 Error`类型
* `T or Panic`类型(可能导致运行时错误)

有两个选项，`pyimport`默认为后者。如果要作为前者导入，请使用
在`pyimport` `exception_type`中指定`Error`(`exception_type: {Error, Panic}`)

## 异常和结果类型

`Result`类型表示可能是错误的值。`Result`的错误处理在几个方面优于异常机制
首先，从类型定义中可以看出子程序可能会报错，实际使用时也很明显

```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

在上面的示例中，仅凭此代码无法判断哪个函数引发了异常。即使回到函数定义，也很难判断函数是否抛出异常

```python
# Erg
try!:
    do!:
        x = foo!()?.bar()
        y = baz!()
        qux!()?
    e =>
        print! e
```

另一方面，在这个例子中，我们可以看到`foo!`和`qux!`会引发错误
确切地说，`y` 也可能是`Result`类型，但您最终必须处理它才能使用里面的值

使用 `Result` 类型的好处不止于此。`Result` 类型也是线程安全的。这意味着错误信息可以(轻松)在并行执行之间传递

## 语境

由于`Error`/`Result`类型本身不会产生副作用，不像异常，它不能有发送位置(Context)等信息，但是如果使用`.context`方法，可以将信息放在`错误`对象。可以添加。`.context`方法是一种使用`Error`对象本身并创建新的 `Error` 对象的方法。它们是可链接的，并且可以包含多个上下文

```python,checker_ignore
f() =
    todo() \
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

请注意，诸如 `.msg` 和 `.kind` 之类的 `Error` 属性不是次要的，因此它们不是上下文，并且不能像最初创建时那样被覆盖

## 堆栈跟踪

`Result` 类型由于其方便性在其他语言中经常使用，但与异常机制相比，它的缺点是难以理解错误的来源
因此，在 Erg 中，`Error` 对象具有名为 `.stack` 的属性，并再现了类似伪异常机制的堆栈跟踪
`.stack` 是调用者对象的数组。每次 Error 对象被`return`(包括通过`?`)时，它都会将它的调用子例程推送到`.stack`
如果它是 `?`ed 或 `.unwrap`ed 在一个不可能 `return` 的上下文中，它会因为回溯而恐慌

```python,checker_ignore
f x =
    ...
    y = foo.try_some(x)?
    ...

g x =
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
# ...
# Foo.try_some, line 10, file "foo.er"
# 10 | y = foo.try_some(x)?
# module::f, line 23, file "foo.er"
# 23 | y = f(x)?
# module::g, line 40, file "foo.er"
# 40 | i = g(1)?
# Error: ...
```

## 恐慌

Erg 还有一种处理不可恢复错误的机制，称为 __panicing__
不可恢复的错误是由外部因素引起的错误，例如软件/硬件故障、严重到无法继续执行代码的错误或程序员未预料到的错误。等如果发生这种情况，程序将立即终止，因为程序员的努力无法恢复正常运行。这被称为"恐慌"

恐慌是通过 `panic` 功能完成的

```python,checker_ignore
panic "something went wrong!"
```

<p align='center'>
    <a href='./30_decorator.md'>上一页</a> | <a href='./32_pipeline.md'>下一页</a>
</p>