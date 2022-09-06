# Inf

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Inf.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Inf.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

Inf是一个类，其唯一实例是inf。
inf的主要用途是用于区间类型。
例如，大于等于 2 的整数类型是 `2..<inf`，小于等于 0 的实数是 `-inf<..0.0`。
由于 inf 不是通常意义上的数字，所以不能按原样定义四个算术运算，
库中提供了所谓的扩展数字类，例如ExtNat。
