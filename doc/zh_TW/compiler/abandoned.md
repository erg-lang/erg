# 廢棄/拒絕的語言規范

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/abandoned.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/abandoned.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

## 重載(臨時多態性)

被放棄了，因為它可以用參數+子類型多態來代替，并且與Python的語義不兼容。 有關詳細信息，請參閱 [overload](../syntax/type/advanced/overloading.md) 文章。

## 具有顯式生命周期的所有權系統

原計劃引入 Rust 之類的所有權系統，但由于與 Python 的語義不兼容以及需要引入生命周期注解等復雜規范而被放棄，并且所有不可變對象都是 RC。托管的可變對象現在只有一個所有權.
Dyne 沒有 C# 和 Nim 那樣的 GIL，策略是允許值對象和低級操作在安全范圍內。