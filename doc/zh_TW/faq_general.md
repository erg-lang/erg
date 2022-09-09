# Erg常見問題

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_general.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_general.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

此常見問題解答適用於一般 Erg 初學者。
對於個別（常見）技術問題，請參閱 [此處](./faq_technical.md) 了解個別（常見）技術問題，以及
[這裡](./dev_guide/faq_syntax.md) 了解更多信息。

## Erg 是 Python 兼容語言是什么意思？

~~A：Erg的可執行系統EVM(Erg VirtualMachine)執行Erg字節碼，是Python字節碼的擴展。它在 Python 字節碼中引入了靜態類型系統和其他特性(例如向不帶參數的指令引入參數，以及在自由編號中實現唯一指令)。這讓 Erg 可以無縫調用 Python 代碼并快速執行。~~

A: Erg 代碼被轉譯成 Python 字節碼。也就是說，它運行在與 Python 相同的解釋器上。最初，我們計劃開發一個兼容 Cpython 的解釋器，并將其與編譯器結合起來形成“Erg”。但是，由于處理系統的發展遠遠落后于編譯器，我們決定提前只發布編譯器(但解釋器仍在開發中)。

## 哪些語言影響了Erg？

我們受到的語言多于我們雙手所能指望的數量，但 Python、Rust、Nim 和 Haskell 的影響最大。
我們從 Python 繼承了許多語義，從 Rust 繼承了面向表達式和 trait，從 Nim 繼承了過程，從 Haskell 繼承了函數式編程相關的特性。

## 可以調用 Python 的語言包括 Julia。你為什么創建 Erg？

答：Erg 設計的動機之一是擁有一種易于使用且具有強大類型系統的語言。即具有類型推斷、Kind、依賴類型等的語言。
Julia 是可以有類型的，但它確實是一種動態類型語言，不具備靜態類型語言的編譯時錯誤檢測優勢。

## Erg 支持多種編程風格，包括函數式和面向對象的編程。這不是與 Python 的“應該有一種——最好只有一種——明顯的方法”相反嗎？

答：在 Erg 中，該術語是在更狹窄的上下文中使用的。例如，Erg API 中一般沒有別名；在這種情況下，Erg是“唯一一種方式”。
在更大的上下文中，例如 FP 或 OOP，只有一種做事方式并不一定很方便。
例如，JavaScript 有幾個庫可以幫助創建不可變的程序，而 C 有幾個用于垃圾收集的庫。
然而，即使是這樣的基本功能也有多個庫不僅需要時間來選擇，而且在集成使用不同庫的代碼時也會產生很大的困難。
即使在純函數式語言 Haskell 中，也有支持 OOP 的庫。
如果程序員沒有一些東西，他們會自己創造它們。因此，我們認為將它們作為標準提供會更好。
這也符合 Python 的“含電池”概念。

## Erg 這個名字的由來是什么？

它以cgs單位系統中的能量單位erg命名。它具有雙重含義：一種為程序員提供能量的符合人體工程學的語言。

還有其他幾個候選者，但之所以選擇它是因為它最短(根據 Ruby 的開發者 Matz 的說法，語言名稱越短越好)并且具有相當高的可搜索性。
