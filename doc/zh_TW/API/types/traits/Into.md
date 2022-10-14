# Into T

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Into.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Into.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

一種類型，表明它可以被類型轉換為類型T
即使Self和T之間沒有繼承關系，也是在關系可以相互轉換的時候定義的
與繼承不同，沒有隱式轉換。您必須始終調用 `.into` 方法

## 方法

* into(self, T) -> T

  変換を行います
