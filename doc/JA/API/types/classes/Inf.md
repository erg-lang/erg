# Inf

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Inf.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Inf.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

Infはinfただひとつをインスタンスとするクラスである。
infの主な使いみちは、区間型での使用である。
例えば、2以上の整数型は`2..<inf`となり、0以下の実数は`-inf<..0.0`となる。
infは通常の意味での数ではないため四則演算をそのままでは定義できないが、
ExtNatなどいわゆる拡大数のクラスがライブラリで提供されている。
