# Erg 部分设计的原因

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_syntax.md%26commit_hash%3D750831b0bdfee37fb71c8e9d315a93040fdea9c9)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_syntax.md&commit_hash=750831b0bdfee37fb71c8e9d315a93040fdea9c9)

## Erg内存管理模型

在CPython后端中使用所有权 + Python内存管理模型(不过Erg代码中的循环引用不会通过GC处理详见[此处](syntax/18_ownership.md/#循环引用))

在Erg自己的虚拟机(Dyne)中使用所有权 + [Perceus](https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v1.pdf)内存管理模型，如果Erg代码使用了Python API那么这些Erg代码使用跟踪垃圾回收内存管理模型

在LLVM, WASM后端使用所有权 + [Perceus](https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v1.pdf)内存管理模型

无论是什么后端都不需要因为内存管理的不同对代码进行任何更改

__注意__: Erg 引入所有权系统的动机不是像 Rust 那样"不依赖 GC 的内存管理"。
Erg 所有权系统的目标是"可变状态的本地化"。 Erg 有一个附属于可变对象的所有权概念。
这是因为共享可变状态容易出现错误，甚至违反类型安全(参见 [此处](../syntax/type/advanced/shared.md#共享参考))。这是一个判断决定。

## 为什么类型参数要大括号 || 而不是 <> 或 []?

这是因为 `<>` 和 `[]` 会导致语法冲突。

```python
# []版
id[T: Type] [t]: [T] = t
y = id[Int] # 这是一个功能吗?
# <>版
id<T: Type> {t: T} = t
y = (id<Int, 1> 1) # 这是一个元组吗?
# {}版
id{T: Type} {t: T} = t
y = id{Int} # 这是一个功能吗?
# ||版
id|T: Type| t: T = t
y = id|Int| # OK
```

## {i=1} 的类型为 {i=Int}，但在 OCaml 等环境中为 {i：Int}。为什么 Erg 采用前者的语法?

Erg 设计为将类型本身也视为值。

```python
A = [Int; 3]
assert A[2] == Int
T = (Int, Str)
assert T.1 == Str
D = {Int: Str}
assert D[Int] == Str
S = {.i = Int}
assert S.i == Int
```

## 你打算在 Erg 中实现宏吗?

目前没有。宏观大致分为四个目的。第一个是编译时计算。这在 Erg 中由编译时函数负责。第二，代码执行的延迟。这可以用 do 块来代替。第三个是处理通用化，对此多相关数和全称类型是比宏观更好的解决方案。第四个是自动生成代码，但这会造成可读性的下降，所以我们不敢在 Erg 中实现。因此，宏的大部分功能都由 Erg 型系统承担，因此没有动力进行部署。

## 为什么 Erg 没有异常机制?

因为在许多情况下，使用 `Result` 类型进行错误处理是更好的解决方案。 `Result` 类型是相对较新的编程语言中使用的常见错误处理技术。

在 Erg 中，`?` 运算符使编写无错误。

```python
read_file!() =
    f = open!("foo.txt")? # 如果失败则立即返回错误，所以 f 是文件类型
    f.read_all!()

# 也可以使用 try 过程捕获类似的异常
try!:
    do!
        s = read_file!()?
        print! s
    e =>
        # 发生错误时执行的块
        print! e
        exit 1
```

在引入 Python 函数时，缺省情况下，所有函数都被视为包含异常，返回类型为。如果你知道不调度异常，请在<gtr="12"/>中指明。

此外，Erg 没有引入异常机制的另一个原因是它计划引入并行编程的功能。这是因为异常机制与并行执行不兼容(例如，如果并行执行导致多个异常，则很难处理)。

## Erg 似乎消除了 Python 被认为是坏做法的功能，但为什么没有取消继承?

Python 的库中有一些类设计为继承，如果完全取消继承，这些操作就会出现问题。然而，由于 Erg 的类默认为 final，并且原则上禁止多重和多层继承，因此继承的使用相对安全。

## 为什么多相关数的子类型推理默认指向记名trait?

默认情况下，指向结构托盘会使类型指定变得复杂，并且可能会混合程序员的非预期行为。

```python
# 如果 T 是结构特征的子类型...
# f: |T <: Structural Trait {.`_+_` = Self.(Self) -> Self; .`_-_` = Self.(Self) -> Self}| (T, T) -> T
f|T| x, y: T = x + y - x
# T 是名义特征的子类型
# g: |T <: Add() and Sub()| (T, T) -> T
g|T| x, y: T = x + y - x
```

## Erg 是否实现了定义自己的运算符的功能?

A：没有那个计划。最重要的原因是，如果允许定义自己的运算符，就会出现如何处理组合顺序的问题。可以定义自己的运算符的 Scala 和 Haskell 等都有不同的对应，但这可以看作是可能产生解释差异的语法的证据。此外，独立运算符还有一个缺点，那就是可能产生可读性较低的代码。

## 为什么 Erg 取消了 += 这样的扩展赋值运算符?

首先，Erg 中没有变量可变性。 换句话说，它不能被重新分配。 一旦一个对象绑定到一个变量，它就会一直绑定到该变量，直到它超出范围并被释放。 Erg 中的可变性意味着对象可变性。 一旦你知道了这一点，故事就很简单了。 例如，`i += 1` 表示 `i = i + 1`，但这样的语法是非法的，因为变量没有被重新分配。 Erg 的另一个设计原则是操作符不应该有副作用。 Python 大多是这样，但是对于某些对象，例如 Dict，扩展赋值运算符会改变对象的内部状态。 这不是一个非常漂亮的设计。
这就是扩展赋值运算符完全过时的原因。

## 为什么 Erg 在语法上特别对待有副作用的过程?

副作用的局部化是代码维护的一个关键因素。

但是，确实也不是没有方法可以不在语言上特殊对待副作用。例如，可以用代数效果(类型系统上的功能)替代过程。但这样的合一并不总是正确的。例如，Haskell 没有对字符串进行特殊处理，只是一个字符数组，但这种抽象是错误的。

什么情况下，可以说合一化是错的?一个指标是"是否会因其合一而难以看到错误信息"。Erg 设计师发现，将副作用特殊处理会使错误消息更容易阅读。

Erg 有一个强大的类型系统，但并不是所有的类型都决定了它。如果这样做了，你的下场就跟 Java 试图用类来控制一切一样。
