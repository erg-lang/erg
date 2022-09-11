# 技术常见问题

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_technical.md%26commit_hash%3Dc120700585fdb1d655255c8e2817bb13cc8d369e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_technical.md&commit_hash=c120700585fdb1d655255c8e2817bb13cc8d369e)

本节回答有关使用 Erg 语言的技术问题。换句话说，它包含以 What 或 Which 开头的问题，以及可以用 Yes/No 回答的问题。

有关如何确定语法的更多信息，请参阅 [此处](./dev_guide/faq_syntax.md) 了解基础语法决策，以及 [此处](./dev_guide/../faq_general.md)。

## Erg 中有异常机制吗?

答：不会。Erg 使用 `Result` 类型代替。请参阅 [此处](./dev_guide/faq_syntax.md) 了解 Erg 没有异常机制的原因。

## Erg 是否有与 TypeScript 的 `Any` 等价的类型?

答：不，没有。所有对象都至少属于 `Object` 类，但是这种类型只提供了一组最小的属性，所以你不能像使用 Any 那样对它做任何你想做的事情。
`Object` 类通过`match` 等动态检查转换为所需的类型。它与Java 和其他语言中的`Object` 是同一种。
在 Erg 世界中，没有像 TypeScript 那样的混乱和绝望，其中 API 定义是“Any”。

## Never、{}、None、()、NotImplemented 和 Ellipsis 有什么区别?

A：`Never` 是一种“不可能”的类型。产生运行时错误的子例程将“Never”(或“Never”的合并类型)作为其返回类型。该程序将在检测到这一点后立即停止。尽管 `Never` 类型在定义上也是所有类型的子类，但 `Never` 类型的对象永远不会出现在 Erg 代码中，也永远不会被创建。 `{}` 等价于 `Never`。
`Ellipsis` 是一个表示省略号的对象，来自 Python。
`NotImplemented` 也来自 Python。它被用作未实现的标记，但 Erg 更喜欢产生错误的 `todo` 函数。
`None` 是 `NoneType` 的一个实例。它通常与 `Option` 类型一起使用。
`()` 是一个单元类型和它自己的一个实例。当您想要返回“无意义的值”(例如过程的返回值)时使用它。

## 为什么 `x = p!()` 有效但 `f() = p!()` 会导致 EffectError?

`!` 不是副作用产品的标记，而是可能导致副作用的对象。
过程 `p!` 和可变类型 `T!` 会引起副作用，但如果 `p!()` 的返回值是 `Int` 类型，它本身就不再引起副作用。

## 当我尝试使用 Python API 时，对于在 Python 中有效的代码，我在 Erg 中收到类型错误。这是什么意思?

A：Erg API 的类型尽可能接近 Python API 规范，但有些情况无法完全表达。
此外，根据规范有效但被认为不合需要的输入(例如，在应该输入 int 时输入浮点数)可能会被 Erg 开发团队酌情视为类型错误。