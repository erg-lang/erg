# Subscript(添字アクセス)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_generator.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_generator.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

`[]`は通常のメソッドとは異なっています。

```erg
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

サブルーチンの戻り値には参照を指定できないということを思い出してください。
`a[0]`の型は、ここでは明らかに`Ref!(Int!)`であるはずです(`a[0]`の型は文脈に依存します)。
よって、`[]`は実際には`.`と同じく特別な構文の一部です。Pythonとは違い、オーバーロードできません。
メソッドで`[]`の挙動を再現することもできません。

```erg
C = Class {i = Int!}
C.get(ref self) =
    self::i # TypeError: `self::i` is `Int!` (require ownership) but `get` doesn't own `self`
C.steal(self) =
    self::i
# NG
C.new({i = 1}).steal().inc!() # OwnershipWarning: `C.new({i = 1}).steal()` is not owned by anyone
# hint: assign to a variable or use `uwn_do!`
# OK (assigning)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

また、`[]`は所有権を奪うこともできますが、その際に要素がシフトするわけではありません。

```erg
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # OwnershipError: `a[0]` is moved to `i`
```
