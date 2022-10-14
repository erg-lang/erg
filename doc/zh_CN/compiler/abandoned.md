# 废弃/拒绝的语言规范

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/abandoned.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/abandoned.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

## 重载(临时多态性)

被放弃了，因为它可以用参数+子类型多态来代替，并且与Python的语义不兼容。有关详细信息，请参阅 [overload](../syntax/type/advanced/overloading.md) 文章

## 具有显式生命周期的所有权系统

原计划引入 Rust 之类的所有权系统，但由于与 Python 的语义不兼容以及需要引入生命周期注解等复杂规范而被放弃，并且所有不可变对象都是 RC。托管的可变对象现在只有一个所有权.
Dyne 没有 C# 和 Nim 那样的 GIL，策略是允许值对象和低级操作在安全范围内。