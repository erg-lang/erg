# Erg常见问题

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_general.md%26commit_hash%3D521426cba21ed8b6eae5aff965dd14ef99af1228)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_general.md&commit_hash=521426cba21ed8b6eae5aff965dd14ef99af1228)

此常见问题解答适用于一般 Erg 初学者。
对于个别(常见)技术问题，请参阅 [此处](./faq_technical.md) 了解个别(常见)技术问题，以及
[这里](./faq_syntax.md) 了解更多信息。

## Erg 是 Python 兼容语言是什么意思?

~~A：Erg的可执行系统EVM(Erg VirtualMachine)执行Erg字节码，是Python字节码的扩展。它在 Python 字节码中引入了静态类型系统和其他特性(例如向不带参数的指令引入参数，以及在自由编号中实现唯一指令)。这让 Erg 可以无缝调用 Python 代码并快速执行。~~

A: Erg 代码被转译成 Python 字节码。也就是说，它运行在与 Python 相同的解释器上。最初，我们计划开发一个兼容 Cpython 的解释器，并将其与编译器结合起来形成"Erg"。但是，由于处理系统的发展远远落后于编译器，我们决定提前只发布编译器(但解释器仍在开发中)。

## 哪些语言影响了Erg?

我们受到的语言多于我们双手所能指望的数量，但 Python、Rust、Nim 和 Haskell 的影响最大。
我们从 Python 继承了许多语义，从 Rust 继承了面向表达式和 trait，从 Nim 继承了过程，从 Haskell 继承了函数式编程相关的特性。

## 已经有一些语言可以调用Python，比如Julia。为什么要创建Erg?

答：Erg 设计的动机之一是拥有一种易于使用且具有强大类型系统的语言。即具有类型推断、Kind、依赖类型等的语言。
Julia 是可以有类型的，但它确实是一种动态类型语言，不具备静态类型语言的编译时错误检测优势。

## Erg 支持多种编程风格，包括函数式和面向对象的编程。这不是与 Python 的"应该有一种——最好只有一种——明显的方法"相反吗?

答：在 Erg 中，该术语是在更狭窄的上下文中使用的。例如，Erg API 中一般没有别名；在这种情况下，Erg是"唯一一种方式"。
在更大的上下文中，例如 FP 或 OOP，只有一种做事方式并不一定很方便。
例如，JavaScript 有几个库可以帮助创建不可变的程序，而 C 有几个用于垃圾收集的库。
然而，即使是这样的基本功能也有多个库不仅需要时间来选择，而且在集成使用不同库的代码时也会产生很大的困难。
即使在纯函数式语言 Haskell 中，也有支持 OOP 的库。
如果程序员没有一些东西，他们会自己创造它们。因此，我们认为将它们作为标准提供会更好。
这也符合 Python 的"含电池"概念。

## Erg 这个名字的由来是什么?

它以cgs单位系统中的能量单位erg命名。它具有双重含义：一种为程序员提供能量的符合人体工程学的语言。

还有其他几个候选者，但之所以选择它是因为它最短(根据 Ruby 的开发者 Matz 的说法，语言名称越短越好)并且具有相当高的可搜索性。
