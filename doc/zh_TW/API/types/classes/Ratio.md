# Ratio

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Ratio.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Ratio.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

表示有理數的類型。 它主要用于當您要使用分數時。
實際上，Erg中的/運算符返回 Ratio。1/3等不被評估為 0.33333... 并且被處理為1/3。 此外，0.1 相當于 1/10。 所以`0.1 + 0.2 == 0.3`。 這聽起來很明顯，但在 Python中它是False。
但是，Ratio類型的效率往往比Float類型略低。 在執行速度很重要且不需要精確數值的地方應該使用浮點類型。 然而，正如Rob Pike所說，過早優化是萬惡之源。 在丟棄Ratio類型并使用Float類型之前，請進行真實的性能測試。 業余愛好者無條件偏愛較輕的模具。
