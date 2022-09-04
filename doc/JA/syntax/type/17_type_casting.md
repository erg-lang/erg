# キャスト

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/17_type_casting.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/17_type_casting.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

## アップキャスト

Pythonはダックタイピングを採用する言語のため、キャストという概念はありません。アップキャストはする必要がなく、ダウンキャストも基本的にはありません。
しかしErgは静的に型付けされるため、キャストを行わなければいけない場合があります。
簡単な例では、`1 + 2.0`が挙げられます。Ergの言語仕様上は`+`(Int, Ratio)、すなわちInt(<: Add(Ratio, Ratio))の演算は定義されていません。というのも、`Int <: Ratio`であるため、1はRatioのインスタンスである1.0にアップキャストされるからです。

~~Erg拡張バイトコードはBINARY_ADDに型情報を加えますが、この際の型情報はRatio-Ratioとなります。この場合はBINARY_ADD命令がIntのキャストを行うため、キャストを指定する特別な命令は挿入されません。なので、例えば子クラスでメソッドをオーバーライドしても、親を型に指定すれば型強制(type coercion)が行われ、親のメソッドで実行されます(コンパイル時に親のメソッドを参照するように名前修飾が行われます)。コンパイラが行うのは型強制の妥当性検証と名前修飾のみです。ランタイムがオブジェクトをキャストすることはありません(現在のところ。実行最適化のためにキャスト命令が実装される可能性はあります)。~~

```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    # オーバーライドする際にはOverrideデコレータが必要
    @Override
    greet!() = print! "Hello from Child"

greet! p: Parent = p.greet!()

parent = Parent.new()
child = Child.new()

greet! parent # "Hello from Parent"
greet! child # "Hello from Parent"
```

この挙動はPythonとの非互換性を生むことはありません。そもそもPythonでは変数に型が指定されないので、いわば全ての変数が型変数で型付けされている状態となります。型変数は適合する最小の型を選ぶので、Ergで型を指定しなければPythonと同じ挙動が達成されます。

```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    greet!() = print! "Hello from Child"

greet! some = some.greet!()

parent = Parent.new()
child = Child.new()

greet! parent # "Hello from Parent"
greet! child # "Hello from Child"
```

継承関係にある型同士では`.from`, `.into`が自動実装されるので、それを使うこともできます。

```erg
assert 1 == 1.0
assert Ratio.from(1) == 1.0
assert 1.into<Ratio>() == 1.0
```

## ダウンキャスト

ダウンキャストは一般に安全ではなく、変換方法も自明ではないため、代わりに`TryFrom.try_from`の実装で実現します。

```erg
IntTryFromFloat = Patch Int
IntTryFromFloat.
    try_from r: Float =
        if r.ceil() == r:
            then: r.ceil()
            else: Error "conversion failed"
```
