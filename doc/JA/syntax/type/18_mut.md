# 可変型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/18_mut.md%26commit_hash%3D60dfd8580acb1a06dec36895295f92e823931a59)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/18_mut.md&commit_hash=60dfd8580acb1a06dec36895295f92e823931a59)

> __Warning__: この項の情報は古く、一部に間違いを含みます。

Ergではデフォルトですべての型が不変型、すなわち内部状態を更新できないようになっています。
しかし可変な型ももちろん定義できます。可変型は`!`を付けて宣言します。

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is \{self::name}. I am \{self::age}."
    inc_age! ref! self = self::name.update! old -> old + 1
```

正確には、可変型・または可変型を含む複合型を基底型とする型は型名の最後に`!`を付けなくてはなりません。`!`を付けない型も同一の名前空間に存在してよく、別の型として扱われます。
上の例では、`.age`属性は可変で、`.name`属性は不変となっています。一つでも可変な属性がある場合、全体として可変型になります。

可変型はインスタンスを書き換えるプロシージャルメソッドを定義できますが、プロシージャルメソッドを持つからと言って可変型になるとは限りません。例えば配列型`[T; N]`には要素をランダムに選ぶ`sample!`メソッドが実装されていますが、これはもちろん配列に破壊的変更を加えたりはしません。

可変型オブジェクトの破壊的操作は、主に`.update!`メソッドを介して行います。`.update!`メソッドは高階プロシージャで、`self`に関数`f`を適用して更新します。

```python
i = !1
i.update! old -> old + 1
assert i == 2
```

`.set!`メソッドは単に古い内容を捨てて新しい値に差し替えます。`.set! x = .update! _ -> x`です。

```python
i = !1
i.set! 2
assert i == 2
```

`.freeze_map`メソッドは値を不変化して操作を行います。

```python
a = [1, 2, 3].into [Nat; !3]
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(Array)
```

多相不変型において型の型引数`T`は暗黙に不変型であると仮定されます。

```python
# ImmutType < Type
K T: ImmutType = Class ...
K! T: Type = Class ...
```

標準ライブラリでは、可変型`(...)!`型は不変型`(...)`型を基底としている場合が多いです。しかし`T!`型と`T`型に言語上特別な関連はなく、そのように構成しなくても構いません[<sup id="f1">1</sup>](#1)。

`T = (...)`のとき単に`T! = (...)!`となる型`(...)`を単純構造型と呼びます。単純構造型は(意味論上)内部構造を持たない型ともいえます。
リスト、タプル、セット、辞書、レコード型は単純構造型ではありませんが、Int型やStr型は単純構造型です。

以上の説明から、可変型とは自身が可変であるものだけでなく、内部に持つ型が可変であるものも含まれるということになります。
`{x: Int!}`や`[Int!; 3]`などの型は、内部のオブジェクトが可変であり、インスタンス自身が可変なわけではない内部可変型です。

## Cell! T

Intやリストなどの不変型に対しては、既に可変型が定義されています。しかし、このような可変型はどのようにして定義されたのでしょうか？例えば、`{x = Int; y = Int}`型に対しては`{x = Int!; y = Int!}`型などが対応する可変型です。
しかし`Int!`型はどうやって`Int`型から作られたのでしょうか？あるいは`Int!`型はどのようにして`Int`型と関係付けられているのでしょうか？

それらに対する答えが`Cell!`型です。`Cell! T`型は`T`型オブジェクトを格納する箱のような型です。

```python
IntOrStr = Inr or Str
IntOrStr! = Cell! IntOrStr
x = IntOrStr!.new 1
assert x is! 1 # `Int or Str` cannot compare with `Int` directly, so use `is!` (this compares object IDs) instead of `==`.
x.set! "a"
assert x is! "a"
```

`Cell! T`型の重要な性質として、`T`型の部分型になるというものがあります。これより、`Cell! T`型のオブジェクトは`T`型のメソッドを全て使うことができます。

```python
# definition of `Int!`
Int! = Cell! Int
...

i = !1
assert i == 1 # `i` is casted to `Int`
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> `T!`型と`T`型に言語上の特別な関係がないのは意図的な設計です。関連があったとすると、例えば名前空間に`T`/`T!`型が存在するときに別のモジュールから`T!`/`T`型を導入できなくなるなどの不都合が生じます。また、不変型に対し可変型は一意に定まりません。`T = (U, V)`という定義があった際、`(U!, V)`と`(U, V!)`という可変サブタイプが`T!`としてあり得えます。[↩](#f1)</span>

<p align='center'>
    <a href='./17_type_casting.md'>Previous</a> | <a href='./19_bound.md'>Next</a>
</p>
