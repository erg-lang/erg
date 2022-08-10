# Projection Type(射影型)

射影型は、次のコードにおける`Self.AddO`のような型を表します。

```erg
Add R, O = Trait {
    .`_+_` = Self, R -> O
}
BinAdd = Subsume Add(Self, Self.AddO), {
    .AddO = Type
}

IntIsBinAdd = Patch(Int, Impl: BinAdd)
IntIsBinAdd.
    AddO = Int
```

`Add(R, O)`型は何らかのオブジェクトとの加算が定義されている型、`BinAdd`は自身のクラスとの加算(閉じた加算)が定義されている型といえます。メソッドは型属性であるべきなので、`+`の型宣言はインデント以下に記述します。
引数のない`Add`型のミソとなるのが`.AddO: Type`という宣言で、これがないと右辺型がエラーになります。

```erg
BinAdd = Add(Self, Self.AddO) # TypeError: trait object 'BinAdd' has no attribute 'AddO'
```

射影型である`.AddO`型の実体は、`Add`のサブタイプである型が持ちます。例えば、`Int.AddO = Int`, `Odd.AddO = Even`です。

```erg
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```

## Appendix: 射影型の型推論

```erg
f x: BinAdd = x + x

f 10
```

上のコードを例にして考えます。
グローバル名前空間には以下の`+`の実装が存在します。

```erg
`_+_`: {(A, R) -> O | R; O; A <: Add(R, O)}
```

`BinAdd = Add(Self, Self.AddO)`なので、`Add`型であるオブジェクトに対しては````_+_`: {(A, A) -> A.AddO | A <: Add(A, A.AddO)}```と置換できます。

```erg
f|A <: BinAdd|(x: A): A.AddO = x + x
```
