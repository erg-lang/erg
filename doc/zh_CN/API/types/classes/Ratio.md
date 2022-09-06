# Ratio

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Ratio.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Ratio.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

表示有理数的类型。 它主要用于当您要使用分数时。
实际上，Erg中的/运算符返回 Ratio。1/3等不被评估为 0.33333... 并且被处理为1/3。 此外，0.1 相当于 1/10。 所以`0.1 + 0.2 == 0.3`。 这听起来很明显，但在 Python中它是False。
但是，Ratio类型的效率往往比Float类型略低。 在执行速度很重要且不需要精确数值的地方应该使用浮点类型。 然而，正如Rob Pike所说，过早优化是万恶之源。 在丢弃Ratio类型并使用Float类型之前，请进行真实的性能测试。 业余爱好者无条件偏爱较轻的模具。
