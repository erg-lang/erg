# 射影型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/projection.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/projection.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

射影型は、次のコードにおける`Self.AddO`のような型を表します。

```python
Add R = Trait {
    .`_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

`Add(R)`型は何らかのオブジェクトとの加算が定義されている型といえます。メソッドは型属性であるべきなので、`+`の型宣言はインデント以下に記述します。
`Add`型のミソとなるのが`.AddO = Type`という宣言で、射影型である`.AddO`型の実体は、`Add`のサブタイプである型が持ちます。例えば、`Int.AddO = Int`, `Odd.AddO = Even`です。

```python
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```
