# デコレータ(修飾子)

デコレータは型や関数に特定の状態や振る舞いを追加したり明示するために使われます。
デコレータの文法は以下の通りです。

```erg
@deco
X = ...
```

デコレータは、競合しない限り複数つけることができます。

デコレータは特別なオブジェクトではなく、その実体は単なる1引数関数です。デコレータは以下の疑似コードと等価です。

```erg
X = ...
X = deco(X)
```

Ergでは変数の再代入が出来ないので、上のようなコードは通らず、デコレータが必要なのです。
以下に、頻出の組み込みデコレータを紹介します。

## Inheritable

定義する型が継承可能クラスであることを示します。引数`scope`に`"public"`を指定すると、外部モジュールのクラスでも継承できるようになります。デフォルトでは`"private"`になっており、外部からは継承できません。

## Final

メソッドをオーバーライド不能にします。クラスに付けると継承不能クラスになりますが、デフォルトなので意味はありません。

## Override

属性をオーバーライドする際に使用します。Ergではデフォルトで基底クラスと同じ属性を定義しようとするとエラーになります。

## Impl

引数のトレイトを実装することを示します。

```erg
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
Sub = Trait {
    .`_-_` = Self.(Self) -> Self
}

C = Class({i = Int}, Impl: Add and Sub)
C.
    @Impl Add
    `_+_` self, other = C.new {i = self::i + other::i}
    @Impl Sub
    `_-_` self, other = C.new {i = self::i - other::}
```

## Attach

トレイトにデフォルトで付属するアタッチメントパッチを指定します。
これによって、Rustのトレイトと同じ挙動を再現できます。

```erg
# foo.er
Add R, O = Trait {
    .`_+_` = Self.(R) -> O
}
@Attach IntIsBinAdd, OddIsBinAdd
BinAdd = Subsume Add(Self, Self.AddO), {
    .AddO = Type
}

IntIsBinAdd = Patch(Int, Impl: BinAdd)
IntIsBinAdd.AddO = Int
OddIsBinAdd = Patch(Odd, Impl: BinAdd)
OddIsBinAdd.AddO = Even
```

こうすると、他のモジュールからトレイトをインポートした際に、アタッチメントパッチが自動で適用されます。

```erg
# 本来IntIsBinAdd, OddIsBinAddも同時にインポートする必要があるが、アタッチメントパッチなら省略可
{BinAdd; ...} = import "foo"

assert Int.AddO == Int
assert Odd.AddO == Even
```

内部的にはトレイトの`.attach`メソッドを使って結びつけているだけです。コンフリクトする場合はトレイトの`.detach`メソッドで外すことができます。

```erg
@Attach X
T = Trait ...
assert X in T.attaches
U = T.detach(X).attach(Y)
assert X not in U.attaches
assert Y in U.attaches
```

## Deprecated

変数の仕様が古く非推奨であることを示します。

## Test

テストサブルーチンであることを示します。テストサブルーチンは`erg test`コマンドで実行されます。

<p align='center'>
    <a href='./28_spread_syntax.md'>Previous</a> | <a href='./30_error_handling.md'>Next</a>
</p>
