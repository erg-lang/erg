# Never

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Never.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Never.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

它是所有類型的子類型。它是一個`Class`，因為它擁有所有的方法，當然還有 `.new`。但是，它沒有實例，并且Erg會在即將創建的那一刻停止
還有一種叫做`Panic`的類型沒有實例，但是`Never`用于正常終止或故意無限循環，`Panic`用于異常終止

```python
# Never <: Panic
f(): Panic = exit 0 # OK
g(): Never = panic() # TypeError
```

`Never`/`Panic`的 OR 類型，例如`T 或 Never`可以轉換為`T`。這是因為`Never`在語義上是一個從不出現的選項(如果出現了，程序會立即停止)
但是，在函數的返回值類型中使用時，`or Never`不能省略，因為它表示程序可能會終止。