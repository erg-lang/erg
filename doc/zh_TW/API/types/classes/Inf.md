# Inf

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Inf.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Inf.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

Inf是一個類，其唯一實例是inf。
inf的主要用途是用于區間類型。
例如，大于等于 2 的整數類型是 `2..<inf`，小于等于 0 的實數是 `-inf<..0.0`。
由于 inf 不是通常意義上的數字，所以不能按原樣定義四個算術運算，
庫中提供了所謂的擴展數字類，例如ExtNat。
