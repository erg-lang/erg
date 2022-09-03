# 值类型（Value types）

值类型是 Erg 内置类型中可以进行编译时评估的类型，具体如下所示。


```erg
Value = (
    Int
    or Nat
    or Ratio
    or Float
    or Complex
    or Bool
    or Str
    or NoneType
    or Array Const
    or Tuple Const
    or Set Const
    or ConstFunc(Const, _)
    or ConstProc(Const, _)
    or ConstMethod(Const, _)
)
```

值类型的对象常量和编译时子例程称为常量表达式。


```erg
1, 1.0, 1+2im, True, None, "aaa", [1, 2, 3], Fib(12)
```

需要注意子程序。子程序可能是值类型，也可能不是。虽然每个子例程的实体都是一个指针，并且都是一个值，但在编译时，在常量上下文中使用非子例程并没有什么意义，因此它不是一个值类型。

以后可能会添加一些类型，这些类型被归类为值类型。

---

<span id="1" style="font-size:x-small">1<gtr=“7”/>Erg 中的值类型一词与其他语言中的定义不同。在纯 Erg 语义学中，内存的概念不存在，因为它被放置在堆栈中，所以它是一种值类型，或者因为它是一个指针，所以它不是一种值类型，这些说法是不正确的。从根本上说，值类型是<gtr=“4”/>类型或其子类型。<gtr=“5”/></span>
