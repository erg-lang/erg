# Rust 代碼指南

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/rust_code_guideline.md%26commit_hash%3D1767df5de23976314a54c3c57bb80be3cb0ddc4f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/rust_code_guideline.md&commit_hash=1767df5de23976314a54c3c57bb80be3cb0ddc4f)

## 本地規則

* 使用 `log!` 進行調試輸出(使用 `println!` 等進行輸出處理，這也是發布所必需的)
* 未使用或內部變量/方法(私有且僅用於特定功能)必須以 `_` 為前綴。如果要避免與保留字沖突，請在末尾添加一個`_`
* 使用clippy。然而，有些規則是不合理的，所以你可以使用`#[allow(clippy::…)]`來忽略除「deny」之外的規則。

## 推薦代碼

* 定義和使用特定領域的枚舉而不是數字枚舉或布爾值
* 將訪問修飾符保持在最低限度。即使在發布時也要優先使用 `pub(mod)` 或 `pub(crate)`
* 將 for 表達式中的可迭代對象顯式轉換為迭代器(`for i in x.iter()` 而不是 `for i in x`)
* 懶惰的評價。例如，如果 `default` 不是文字，請使用 `unwrap_or_else` 而不是 `unwrap_or`
* Use assertions such as `debug_assert!`, `debug_assert_eq!`, `debug_power_assert!`, etc. Specify error messages such as `debug_assert!(... , "{x} is not ...") ;`.

## 不鼓勵使用代碼

* 大量使用返回類型重載。特別是使用大量非顯而易見的 `.into` 的代碼。這是因為類型推斷結果可能違反直覺。在這種情況下，建議使用 `from` 代替
* 大量使用 `Deref`。這有效地提出了與繼承相同的問題

## 根據上下文做出決策的代碼

* 定義未使用的輔助方法
* 大量使用 `unwrap` 和 `clone`。在某些情況下，沒有什麼比這樣做更好的了。

## 依賴關系

依賴關系應該盡可能地最小化，那些必要的依賴關系應該由Erg開發團隊來實現。只有當外部依賴很難實現或依賴于硬件時才允許使用。例如：`libc`， `winapi`)，或者沒有外部依賴的crate(例如:`unicode-xid`)。否則，它們可能被允許作為可選依賴項(例如https客戶端)。在任何情況下，都應選擇保養良好和廣泛使用的

此規則僅適用于Erg編譯器, Erg工具和庫可以自由添加它們自己的依賴項。