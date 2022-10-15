# 投影类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/projection.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/projection.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

投影类型表示如下代码中的"Self.AddO"等类型

```python
Add R = Trait {
    . `_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

类型"Add(R)"可以说是定义了与某个对象的加法的类型。由于方法应该是一个类型属性，`+` 类型声明应该写在缩进下面
`Add` 类型的场景是声明 `.AddO = Type`，而 `.AddO` 类型的实体是一个投影类型，由一个作为 ` 子类型的类型持有 添加`。例如，`Int.AddO = Int`、`Odd.AddO = Even`

```python
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```
