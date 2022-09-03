# 声明

声明是指定要使用的变量类型的语法。可以在代码中的任何地方声明，但不能只声明变量。必须始终初始化。赋值后声明可以检查类型是否与赋值对象匹配。


```erg
i: Int
# i: Int = 2可以与赋值同时声明
i = 2
i: Num
i: Nat
i: -2..2
i: {2}
```

赋值后声明类似于类型检查，但在编译时进行检查。运行时使用<gtr=“6”/>进行类型检查可以用“可能是 XX”进行检查，但编译时使用<gtr=“7”/>进行类型检查是严格的。如果没有确定是“某某型”，就无法通过检查，就会出现错误。


```erg
i = (-1..10).sample!()
assert i in Nat # 这可能会通过
i: Int # 这通过了
i: Nat # 这不起作用（因为 -1 不是 Nat 的元素）
```

可以通过两种方式声明函数。


```erg
f: (x: Int, y: Int) -> Int
f: (Int, Int) -> Int
```

如果显式声明参数名称，则在定义时如果名称不同，将导致类型错误。如果你想给出参数名称的任意性，可以使用第二种方法声明它。在这种情况下，类型检查只显示方法名称及其类型。


```erg
T = Trait {
    .f = (x: Int, y: Int): Int
}

C = Class(U, Impl := T)
C.f(a: Int, b: Int): Int = ... # TypeError: `.f` must be type of `(x: Int, y: Int) -> Int`, not `(a: Int, b: Int) -> Int`
```

<p align='center'>
    <a href='./02_name.md'>Previous</a> | <a href='./04_function.md'>Next</a>
</p>
