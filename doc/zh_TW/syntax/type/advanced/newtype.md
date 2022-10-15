# 新類型模式

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/newtype.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/newtype.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

這是 Rust 中常用的 newtype 模式的 Erg 版本

Erg 允許定義類型別名如下，但它們只引用相同的類型

```python
UserID = Int
```

因此，例如，如果你有一個規范，類型為 `UserId` 的數字必須是一個正的 8 位數字，你可以輸入 `10` 或 `-1`，因為它與類型 `Int` 相同 . 如果設置為 `Nat`，則可以拒絕 `-1`，但 8 位數字的性質不能僅用 Erg 的類型系統來表達

此外，例如，在設計數據庫系統時，假設有幾種類型的 ID: 用戶 ID、產品 ID、產品 ID 和用戶 ID。如果 ID 類型的數量增加，例如用戶 ID、產品 ID、訂單 ID 等，可能會出現將不同類型的 ID 傳遞給不同函數的 bug。即使用戶 ID 和產品 ID 在結構上相同，但它們在語義上是不同的

對于這種情況，newtype 模式是一個很好的設計模式

```python
UserId = Class {id = Nat}
UserId.
    new id: Nat =
        assert id.dights().len() == 8, else: "UserId 必須是長度為 8 的正數"
        UserId::__new__ {id;}

i = UserId.new(10000000)
print! i # <__main__.UserId object>
i + UserId.new(10000001) # TypeError: + is not implemented between `UserId` and `UserId
```

構造函數保證 8 位數字的前置條件
`UserId` 失去了 `Nat` 擁有的所有方法，所以每次都必須重新定義必要的操作
如果重新定義的成本不值得，最好使用繼承。另一方面，在某些情況下，方法丟失是可取的，因此請根據情況選擇適當的方法
