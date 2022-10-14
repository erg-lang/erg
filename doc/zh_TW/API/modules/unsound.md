# 模塊 `unsound`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/unsound.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/unsound.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

讓 API 執行在 Erg 的類型系統中無法保證的不健全和不安全的操作

## `unsafe!`

執行"不安全"過程。就像 Rust 一樣，`Unsafe` API 不能直接調用，而是作為高階函數傳遞給這個過程

```python
unsound = import "unsound"

i = unsound. unsafe! do!:
     # 將 `Result Int` 轉換為 `Int`
     unsound.transmute input!().try_into(Int), Int
```

## transmit

將第一個參數的對象轉換為第二個參數的類型。沒有進行類型檢查
這個函數破壞了類型系統的類型安全。請在使用前進行驗證

## 隱式轉換

與 `transmute` 不同，它會自動轉換為預期的類型。與 Ocaml 的 `Obj.magic` 工作方式相同。