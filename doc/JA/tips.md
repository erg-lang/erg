# Tips

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tips.md%26commit_hash%3D157f51ae0e8cf3ceb45632b537ebe3560a5500b7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tips.md&commit_hash=157f51ae0e8cf3ceb45632b537ebe3560a5500b7)

## エラーの表示言語を変えたい

各国語版のergをダウンロードしてください。
ただし、標準ライブラリ以外では多言語対応がなされていない可能性があります。

## レコードの特定の属性だけ可変化したい

```python
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; rest; ...} = record
mut_record = {.height = !height; ...rest}
```

## 変数のシャドーイングがしたい

Ergで同一スコープ内でのシャドーイングはできません。しかし、スコープが変われば定義しなおせるので、インスタントブロックを使うといいでしょう。

```python
# T!型オブジェクトを取得し、最終的にT型として変数へ代入する
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
```

## final class(継承不能クラス)を何とかして再利用したい

ラッパークラスを作りましょう。これはいわゆるコンポジション(合成)と言われるパターンです。

```python
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## 文字列でない列挙型が使いたい

以下のようにして、他の言語でよく見られる伝統的な列挙型(代数的データ型)を定義できます。
`Singleton`を実装すると、クラスとインスタンスが同一視されます。
また、`Enum`を使うと、その選択肢となる型がリダイレクト属性として自動的に定義されます。

```python
Ok = Class Impl := Singleton
Err = Class Impl := Singleton
ErrWithInfo = Inherit {info = Str}
Status = Enum Ok, Err, ErrWithInfo
stat: Status = Status.new ErrWithInfo.new {info = "error caused by ..."}
match! stat:
    Status.Ok -> ...
    Status.Err -> ...
    Status.ErrWithInfo::{info;} -> ...
```

```python
Status = Enum Ok, Err, ErrWithInfo
# 以下のと同じ
Status = Class Ok or Err or ErrWithInfo
Status.
    Ok = Ok
    Err = Err
    ErrWithInfo = ErrWithInfo
```

## 1始まりでenumerateしたい

method 1:

```python
arr = [...]
for! arr.iter().enumerate(start: 1), i =>
    ...
```

method 2:

```python
arr = [...]
for! arr.iter().zip(1..), i =>
    ...
```

## 非公開APIを(ホワイトボックス)テストしたい

`foo.er`の非公開APIは`foo.test.er`というモジュールでは特別にアクセス可能となります。
`foo.test.er`モジュールはインポートできないので、隠蔽性は保たれます。

```python
# foo.er
private x = ...
```

```python
# foo.test.er
foo = import "foo"

@Test
'testing private' x =
    ...
    y = foo::private x
    ...
```

## 外部からはread-onlyな(可変)属性を定義したい

属性をプライベートにして、ゲッタを定義するとよいでしょう。

```python
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## 引数名を型システム上で識別させたい

引数をレコードで受け取ると良いでしょう。

```python
Point = {x = Int; y = Int}

norm: Point -> Int
norm({x: Int; y: Int}): Int = x**2 + y**2
assert norm({x = 1; y = 2}) == norm({y = 2; x = 1})
```

## 警告を出さないようにしたい

Ergに警告を止めるオプションはありません(これは意図的な設計です)。コードを書き直してください。
