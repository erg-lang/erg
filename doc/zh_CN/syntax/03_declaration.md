# 宣言(Declaration)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/03_declaration.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/03_declaration.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

声明是用于指定要使用的变量类型的语法
可以在代码中的任何地方进行声明，但单独的声明并不引用变量。它们必须被初始化
分配后，可以检查声明以确保类型与分配它的对象兼容

```python
i: Int
# 可以与赋值同时声明，如 i: Int = 2
i = 2
i: Num
i: Nat
i: -2..2
i: {2}
```

赋值后的声明类似于`assert`的类型检查，但具有在编译时检查的特点
在运行时通过`assert`进行类型检查可以检查"可能是Foo类型"，但是在编译时通过`:`进行类型检查是严格的: 如果类型未确定为"类型Foo"，则不会通过 检查会出现错误

```python
i = (-1..10).sample!
assert i in Nat # 这可能会通过
i: Int # 这会通过
i: Nat # 这不会通过(-1 不是 Nat 的元素)
```

函数可以用两种不同的方式声明

```python
f: (x: Int, y: Int) -> Int
f: (Int, Int) -> Int
```

如果显式声明参数名称，如果在定义时名称不同，则会导致类型错误。如果你想给参数名称任意命名，你可以用第二种方式声明它们。在这种情况下，类型检查只会看到方法名称及其类型

```python
T = Trait {
    .f = (x: Int, y: Int): Int
}

C = Class(U, Impl := T)
C.f(a: Int, b: Int): Int = ... # 类型错误: `.f` 必须是 `(x: Int, y: Int) -> Int` 的类型，而不是 `(a: Int, b: Int) -> Int`
```

<p align='center'>
    <a href='./02_name.md'>上一页</a> | <a href='./04_function.md'>下一页</a>
</p>
