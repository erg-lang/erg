# 測試

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/test.md%26commit_hash%3D3e4251b9f9929891dd8ce422c1ed6853f77ab432)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/test.md&commit_hash=3e4251b9f9929891dd8ce422c1ed6853f77ab432)

測試是確保代碼質量的重要部分

使用以下命令執行測試

``` sh
cargo test --features large_thread
```

由于cargo需要一個小線程來運行測試，我們使用 `large_thread` 標志來避免堆棧溢出

## 放置測試

根據實現的特性來安排它們。將解析器測試放置在`erg_parser/tests`下，將編譯器(類型檢查器等)測試放置在`erg_compiler/tests`下，將用戶可以直接使用的語言特性測試放置在`erg/tests`下(然而，這些測試目前正在開發中，不一定按照這種慣例安排)

## 如何編寫測試

有兩種類型的測試。positive測試和negative測試。
positive測試是檢查編譯器是否按預期運行的測試，而negative測試是檢查編譯器是否正確地輸出無效輸入的錯誤。
由于編程語言處理器的性質，在所有軟件中，它們特別容易受到無效輸入的影響，并且必須始終將錯誤呈現給用戶，因此后者也必須得到照顧。

如果你在語言中添加了一個新特性，你至少需要寫一個positive測試。另外，如果可能的話，請寫同時編寫negative測試。