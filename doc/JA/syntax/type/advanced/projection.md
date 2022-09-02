# Projection Type(射影型)

射影型は、次のコードにおける`Self.AddO`のような型を表します。

```erg
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

```erg
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```
