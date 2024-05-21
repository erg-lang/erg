# 属性の解決

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/attribute_resolution.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/attribute_resolution.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

属性の解決とは、例えば`x.y`という式が与えられたときにこの式全体の型を決定することを指します。従って`x`の型を決定する必要がありますが、`x`の型は一意に決定できない場合があります。そのような場合でも`x.y`の型は決定できる場合がありますし、失敗する場合もあります。これが本項が扱う属性の解決の問題です。

簡単な場合として、`1.real`(`x == 1, y == real`)という式を考えてみましょう。`1`の型は`{1}`です。`{1}`は`Nat`や`Int`、`Obj`の部分型です。これらの型を順番に辿って、`real`の定義を探します。この場合は`Int`で見つかります(`Int.real: Int`)。従って`x`の型は`Int`にキャストされ、`1.real`の型は`Int`となります。

このように、`x`の型が一意に特定出来る場合は左辺`x`->右辺`y`という順番で推論が進みます。
しかし`x`の型が特定できないとき、逆に`y`から`x`の型が絞られることもあります。

例えば、このような場合です。

```erg
consts c = c.co_consts
```

`co_consts`は`Code`型の属性です。この関数の意味するところは本質ではなく、単に他と被らない名前であるからこの例を選択しました。
`c`の型が指定されていないので、一見推論はできないように見えますが、(名前空間中に`co_consts`を持つ型が`Code`しかないときは)可能です。

Ergでは変数の型が指定されていないとき、型変数が割り当てられます。

```erg
consts: ?1
c: ?2
```

型推論器は`?2`型から`co_consts`の所属を特定しようとしますが、`?2`は何の条件もついていない型変数なので、失敗します。
このような場合、[`get_attr_type_by_name`](https://github.com/erg-lang/erg/blob/b8a87c0591e5603c1afcfc54c073ab2101ff2857/crates/erg_compiler/context/inquire.rs#L2884)というメソッドが呼ばれます。
このメソッドでは、これまでとは逆に、`co_consts`という名前から`?2`の型を特定しようとします。
これが成功するのは、名前空間中に`co_consts`を持つ型が`Code`しかないときのみです。
Ergでは関数の型検査はモジュール内で閉じているので、モジュール外で`co_consts`を属性に持つ型が定義されていても、そのインスタンスを`consts`関数に渡すとエラーになります(それを可能にするためには、後述する`Structural`を使う必要があります)。この制約によって`consts`関数の推論が可能になります。

型推論器は、クラス属性が定義されるとき、その"属性"と"定義クラス、属性の型"のペアを記録しておきます。
`co_consts`の場合は`{co_consts: {Code, List(Obj, _)}}`というペアです。

```erg
method_to_classes: {co_consts: [{Code, List(Obj, _)}], real: [{Int, Int}], times!: [{Nat, (self: Nat, proc!: () => NoneType) => NoneType}], ...}
```

key-valueペアのvalueが配列になっていることに注意してください。この配列が長さ1であるとき、または(部分型関係による)最小の要素が存在するときのみ、keyは一意に特定できたということになります(そうでなければ型エラーが発生します)。

keyが特定できたら、その定義型を`?2`の型に逆伝搬させます。

```erg
?2(<: Code).co_consts: List(Obj, _)
```

最終的に、`consts`の型は`Code -> List(Obj, _)`となります。

```erg
consts(c: Code): List(Obj, _) = c.co_consts
```
