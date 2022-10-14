# 投影類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/projection.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/projection.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

投影類型表示如下代碼中的"Self.AddO"等類型

```python
Add R = Trait {
    . `_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

類型"Add(R)"可以說是定義了與某個對象的加法的類型。 由于方法應該是一個類型屬性，`+` 類型聲明應該寫在縮進下面
`Add` 類型的場景是聲明 `.AddO = Type`，而 `.AddO` 類型的實體是一個投影類型，由一個作為 ` 子類型的類型持有 添加`。 例如，`Int.AddO = Int`、`Odd.AddO = Even`

```python
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```
