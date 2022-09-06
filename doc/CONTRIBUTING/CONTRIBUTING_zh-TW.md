# 為Erg做貢獻

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3DCONTRIBUTING.md%26commit_hash%3Da86bd4cd1bef4035a1ad23676c8324ab74f7b674)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=CONTRIBUTING.md&commit_hash=a86bd4cd1bef4035a1ad23676c8324ab74f7b674)

初學者應閱讀說明 [此處](https://github.com/erg-lang/erg/issues/31#issuecomment-1217505198)。

## 文件

如果您正在考慮為 Erg 做貢獻，您應該閱讀 [doc/dev_guide](./doc/EN/dev_guide/) 下的文檔。
或者您對 Erg 的內部結構感興趣，[doc/compiler](/doc/JA/compiler/) 可能會提供有用的信息（目前只有日語）。

## 錯誤報告

如果您發現任何您認為是 Erg 中的錯誤的行為，如果您願意 [report](https://github.com/erg-lang/erg/issues/new/choose)，我將不勝感激。請確保尚未將相同的錯誤報告為問題。

如果你輸入 `cargo run --features debug`，Erg 將在調試模式下構建。此模式可能會轉儲可能對調查錯誤有用的信息。如果您能在此模式下報告錯誤日誌，我將不勝感激。

此外，如果錯誤明確不是由環境引起的，則不需要報告錯誤發生的環境。

## 文檔翻譯

我們一直在尋找將我們的文件翻譯成各種語言版本的人。

我們也歡迎那些發現文檔與其他語言相比已經過時並希望更新內容的人（請參閱[此處](https://github.com/erg-lang/erg/issues/48#issuecomment-1218247362)如何做到這一點）。

## 提問

如果您有任何問題，請隨時在 [Discord 頻道](https://discord.gg/zfAAUbgGr4) 上提問。

## 開發

請求總是受歡迎的，但請記住，它們不會總是被接受。許多問題都有取捨。

不要攔截其他人已分配的問題（檢查 GitHub 上的受理人）。如果認為一個人處理起來太困難，我們會呼籲更多的支持。

在提出新功能之前，請考慮通過組合現有功能是否可以輕鬆解決該功能。

請以 Erg 團隊和語言標準化的風格編寫代碼。

## [行為準則](./CODE_OF_CONDUCT.md)
