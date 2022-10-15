# デコレータ(修飾子)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_decorator.md%26commit_hash%3Db07c17708b9141bbce788d2e5b3ad4f365d342fa)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_decorator.md&commit_hash=b07c17708b9141bbce788d2e5b3ad4f365d342fa)

デコレータは型や関数に特定の状態や振る舞いを追加したり明示するために使われます。
デコレータの文法は以下の通りです。

```python
@deco
X = ...
```

デコレータは、競合しない限り複数つけることができます。

デコレータは特別なオブジェクトではなく、その実体は単なる1引数関数です。デコレータは以下の疑似コードと等価です。

```python
X = ...
X = deco(X)
```

Ergでは変数の再代入が出来ないので、上のようなコードは通りません。
単なる変数の場合は`X = deco(...)`と同じなのですが、インスタントブロックやサブルーチンの場合はそうすることができないので、デコレータが必要になってきます。

```python
@deco
f x =
    y = ...
    x + y

# コードが横長になるのを防ぐこともできる
@LongNameDeco1
@LongNameDeco2
C = Class ...
```

以下に、頻出の組み込みデコレータを紹介します。

## Inheritable

定義する型が継承可能クラスであることを示します。引数`scope`に`"public"`を指定すると、外部モジュールのクラスでも継承できるようになります。デフォルトでは`"private"`になっており、外部からは継承できません。

## Final

メソッドをオーバーライド不能にします。クラスに付けると継承不能クラスになりますが、デフォルトなので意味はありません。

## Override

属性をオーバーライドする際に使用します。Ergではデフォルトで基底クラスと同じ属性を定義しようとするとエラーになります。

## Impl

引数のトレイトを実装することを示します。

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
Sub = Trait {
    .`_-_` = Self.(Self) -> Self
}

C = Class({i = Int}, Impl := Add and Sub)
C.
    @Impl Add
    `_+_` self, other = C.new {i = self::i + other::i}
    @Impl Sub
    `_-_` self, other = C.new {i = self::i - other::}
```

## Attach

トレイトにデフォルトで付属するアタッチメントパッチを指定します。
これによって、Rustのトレイトと同じ挙動を再現できます。

```python
# foo.er
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
@Attach AddForInt, AddForOdd
ClosedAdd = Subsume Add(Self)

AddForInt = Patch(Int, Impl := ClosedAdd)
AddForInt.AddO = Int
AddForOdd = Patch(Odd, Impl := ClosedAdd)
AddForOdd.AddO = Even
```

こうすると、他のモジュールからトレイトをインポートした際に、アタッチメントパッチが自動で適用されます。

```python
# 本来IntIsBinAdd, OddIsBinAddも同時にインポートする必要があるが、アタッチメントパッチなら省略できる
{BinAdd; ...} = import "foo"

assert Int.AddO == Int
assert Odd.AddO == Even
```

内部的にはトレイトの`.attach`メソッドを使って結びつけているだけです。コンフリクトする場合はトレイトの`.detach`メソッドで外すことができます。

```python
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

テスト用サブルーチンであることを示します。テスト用サブルーチンは`erg test`コマンドで実行されます。

<p align='center'>
    <a href='./28_spread_syntax.md'>Previous</a> | <a href='./30_error_handling.md'>Next</a>
</p>
