# 技術常見問題

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_technical.md%26commit_hash%3D1b3d7827bb770459475e4102c6f5c43d8ad79ae4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_technical.md&commit_hash=1b3d7827bb770459475e4102c6f5c43d8ad79ae4)

本節回答有關使用 Erg 語言的技術問題。換句話說，它包含以 What 或 Which 開頭的問題，以及可以用 Yes/No 回答的問題

有關如何確定語法的更多信息，請參閱 [此處](./faq_syntax.md) 了解基礎語法決策，以及 [此處](./faq_general.md)

## Erg 中有異常機制嗎?

答: 不會。Erg 使用 `Result` 類型代替。請參閱 [此處](./faq_syntax.md) 了解 Erg 沒有異常機制的原因

## Erg 是否有與 TypeScript 的 `Any` 等價的類型?

答: 不，沒有。所有對象都至少屬于 `Object` 類，但是這種類型只提供了一組最小的屬性，所以你不能像使用 Any 那樣對它做任何你想做的事情
`Object` 類通過`match` 等動態檢查轉換為所需的類型。它與Java 和其他語言中的`Object` 是同一種
在 Erg 世界中，沒有像 TypeScript 那樣的混亂和絕望，其中 API 定義是"Any"

## Never、{}、None、()、NotImplemented 和 Ellipsis 有什么區別?

A: `Never` 是一種"不可能"的類型。產生運行時錯誤的子例程將"Never"(或"Never"的合并類型)作為其返回類型。該程序將在檢測到這一點后立即停止。盡管 `Never` 類型在定義上也是所有類型的子類，但 `Never` 類型的對象永遠不會出現在 Erg 代碼中，也永遠不會被創建。`{}` 等價于 `Never`
`Ellipsis` 是一個表示省略號的對象，來自 Python
`NotImplemented` 也來自 Python。它被用作未實現的標記，但 Erg 更喜歡產生錯誤的 `todo` 函數
`None` 是 `NoneType` 的一個實例。它通常與 `Option` 類型一起使用
`()` 是一個單元類型和它自己的一個實例。當您想要返回"無意義的值"(例如過程的返回值)時使用它

## 為什么 `x = p!()` 有效但 `f() = p!()` 會導致 EffectError?

`!` 不是副作用產品的標記，而是可能導致副作用的對象
過程 `p!` 和可變類型 `T!` 會引起副作用，但如果 `p!()` 的返回值是 `Int` 類型，它本身就不再引起副作用

## 當我嘗試使用 Python API 時，對于在 Python 中有效的代碼，我在 Erg 中收到類型錯誤。這是什么意思?

A: Erg API 的類型盡可能接近 Python API 規范，但有些情況無法完全表達
此外，根據規范有效但被認為不合需要的輸入(例如，在應該輸入 int 時輸入浮點數)可能會被 Erg 開發團隊酌情視為類型錯誤。