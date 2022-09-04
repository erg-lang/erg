# 过载

Erg 不支持。也就是说，不能对函数卡印进行多重定义（过载）。但是，通过组合trait类和补丁，可以再现过载的行为。可以使用trait而不是trait类，但在这种情况下，安装<gtr=“8”/>的所有类型都成为对象。


```erg
Add1 = Trait {
    .add1: Self.() -> Self
}
IntAdd1 = Patch Int, Impl := Add1
IntAdd1.
    add1 self = self + 1
RatioAdd1 = Patch Ratio, Impl := Add1
RatioAdd1.
    add1 self = self + 1.0

add1|X <: Add1| x: X = x.add1()
assert add1(1) == 2
assert add1(1.0) == 2.0
```

这种通过接受某一类型的所有亚型而产生的多相称为。Erg 中的亚分型多相也包括列多相。

如果各型的处理完全相同，也可以写如下。上面的写法用于不同类的行为（但返回类型相同）。使用类型参数的多相称为。参数多相与如下所示的部分型指定并用的情况较多，这种情况下是参数多相和子分型多相的组合技术。


```erg
add1|T <: Int or Str| x: T = x + 1
assert add1(1) == 2
assert add1(1.0) == 2.0
```

另外，自变量数不同类型的过载可以用默认自变量再现。


```erg
C = Class {.x = Int; .y = Int}
C.
    new(x, y := 0) = Self::__new__ {.x; .y}

assert C.new(0, 0) == C.new(0)
```

虽然无法定义根据自变量的数量类型不同等行为完全变化的函数，但 Erg 采取的立场是，如果行为本来就不同，就应该赋予其他名称。

结论是，Erg 禁止过载而采用亚分 + 参数多相是出于以下原因。

首先，被超载的函数的定义是分散的。因此，发生错误时很难报告原因所在。另外，通过导入子程序，可能会改变已经定义的子程序的行为。


```erg
{id; ...} = import "foo"
...
id x: Int = x
...
id x: Ratio = x
...
id "str" # TypeError: id is not implemented for Str
# But... where did this error come from?
```

其次，与默认参数不匹配。当有默认参数的函数被重载时，存在哪个优先的问题。


```erg
f x: Int = ...
f(x: Int, y := 0) = ...

f(1) # which is chosen?
```

再者，与宣言不相匹配。声明无法确定指的是哪一个定义。因为<gtr=“13”/>和<gtr=“14”/>没有包含关系。


```erg
f: Num -> Num
f(x: Int): Ratio = ...
f(x: Ratio): Int = ...
```

而且，破坏语法的连贯性。虽然 Erg 禁止变量的再代入，但是过载的语法看起来像是再代入。也不能替换为无名函数。


```erg
# same as `f = x -> body`
f x = body

# same as... what?
f x: Int = x
f x: Ratio = x
```
