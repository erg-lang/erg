# Rust 代码指南

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/rust_code_guideline.md%26commit_hash%3D1767df5de23976314a54c3c57bb80be3cb0ddc4f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/rust_code_guideline.md&commit_hash=1767df5de23976314a54c3c57bb80be3cb0ddc4f)

## 本地规则

* 使用 `log!` 进行调试输出(使用 `println!` 等进行输出处理，这也是发布所必需的)
* 未使用或内部变量/方法(私有且仅用于特定功能)必须以 `_` 为前缀。如果要避免与保留字冲突，请在末尾添加一个`_`
* 使用clippy。然而，有些规则是不合理的，所以你可以使用`#[allow(clippy::…)]`来忽略除「deny」之外的规则。

## 推荐代码

* 定义和使用特定领域的枚举而不是数字枚举或布尔值
* 将访问修饰符保持在最低限度。即使在发布时也要优先使用 `pub(mod)` 或 `pub(crate)`
* 将 for 表达式中的可迭代对象显式转换为迭代器(`for i in x.iter()` 而不是 `for i in x`)
* 懒惰的评价。例如，如果 `default` 不是文字，请使用 `unwrap_or_else` 而不是 `unwrap_or`
* Use assertions such as `debug_assert!`, `debug_assert_eq!`, `debug_power_assert!`, etc. Specify error messages such as `debug_assert!(... , "{x} is not ...") ;`.

## 不鼓励使用代码

* 大量使用返回类型重载。特别是使用大量非显而易见的 `.into` 的代码。这是因为类型推断结果可能违反直觉。在这种情况下，建议使用 `from` 代替
* 大量使用 `Deref`。这有效地提出了与继承相同的问题

## 根据上下文做出决策的代码

* 定义未使用的辅助方法
* 大量使用 `unwrap` 和 `clone`。在某些情况下，没有什麽比这样做更好的了。

## 依赖关系

依赖关系应该尽可能地最小化，那些必要的依赖关系应该由Erg开发团队来实现。只有当外部依赖很难实现或依赖于硬件时才允许使用。(例如：`libc`， `winapi`)，或者没有外部依赖的crate(例如:`unicode-xid`)。否则，它们可能被允许作为可选依赖项(例如https客户端)。在任何情况下，都应选择保养良好和广泛使用的

此规则仅适用于Erg编译器, Erg工具和库可以自由添加它们自己的依赖项。