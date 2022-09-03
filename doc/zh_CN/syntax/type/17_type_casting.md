# 铸造

## 上投

Python 没有 cast 的概念，因为它采用烤鸭打字的语言。不需要上播，基本上也没有下播。但是，由于 Erg 是静态输入的，因此可能需要强制转换。一个简单的例子是。Erg 的语言规范没有定义<gtr=“6”/>（Int，Ratio），即 Int（ <：Add（Ratio，Ratio））的运算。这是因为<gtr=“7”/>将 1 上传到 Ratio 的实例 1.0。

~~Erg 扩展字节码将类型信息添加到 BINARY_ADD 中，其中类型信息为 Ratio-Ratio。在这种情况下，BINARY_ADD 指令将转换 Int，因此不会插入指定转换的特殊指令。因此，例如，如果在子类中覆盖了某个方法，但将父项指定为类型，则会强制类型（type coercion）并在父项方法中执行（在编译时进行名称限定以引用父项方法）。编译器只执行强制类型验证和名称限定。运行时不会强制转换对象（当前）。可能会实现强制转换指令以进行执行优化。~~


```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    # オーバーライドする際にはOverrideデコレータが必要
    @Override
    greet!() = print! "Hello from Child"

greet! p: Parent = p.greet!()

parent = Parent.new()
child = Child.new()

greet! parent # "Hello from Parent"
greet! child # "Hello from Parent"
```

此行为不会导致与 Python 的不兼容。Python 最初不为变量指定类型，因此所有变量都以类型变量输入。由于类型变量选择最小匹配类型，因此如果 Erg 不指定类型，则会实现与 Python 相同的行为。


```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    greet!() = print! "Hello from Child"

greet! some = some.greet!()

parent = Parent.new()
child = Child.new()

greet! parent # "Hello from Parent"
greet! child # "Hello from Child"
```

对于具有继承关系的类型，和<gtr=“9”/>是自动实现的，你也可以使用它们。


```erg
assert 1 == 1.0
assert Ratio.from(1) == 1.0
assert 1.into<Ratio>() == 1.0
```

## 下铸

降播通常是不安全的，转换方式也不是显而易见的，而是通过实现来实现。


```erg
IntTryFromFloat = Patch Int
IntTryFromFloat.
    try_from r: Float =
        if r.ceil() == r:
            then: r.ceil()
            else: Error "conversion failed"
```
