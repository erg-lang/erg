# オーバーロード

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/overloading.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/overloading.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

Ergでは __アドホック多相__ をサポートしない。すなわち、関数・カインドの多重定義(オーバーロード)ができない。が、トレイトクラスとパッチを組み合わせることでオーバーロードの挙動を再現できる。
トレイトクラスのかわりにトレイトを使用しても良いが、その場合`.add1`を実装している型全てが対象になってしまう。

```python
Add1 = Trait {
    .add1: Self.() -> Self
}
IntAdd1 = Patch Int, Impl := Add1
IntAdd1.
    add1 self = self + 1
RatioAdd1 = Patch Ratio, Impl := Add1
RatioAdd1.
    add1 self = self + 1.0

add1|X <: Add1| x: X = x.add1()
assert add1(1) == 2
assert add1(1.0) == 2.0
```

このような、ある型のサブタイプすべてを受け入れることによる多相を __サブタイピング多相__ と呼ぶ。Ergにおけるサブタイピング多相は列多相も含む。

各型での処理が完全に同じなら下のように書くこともできる。上の書き方は、クラスによって挙動を変える(が、戻り値型は同じ)場合に使う。
型引数を使う多相を __パラメトリック多相__ という。パラメトリック多相は下のように部分型指定と併用する場合が多く、その場合はパラメトリック多相とサブタイピング多相の合わせ技ということになる。

```python
add1|T <: Int or Str| x: T = x + 1
assert add1(1) == 2
assert add1(1.0) == 2.0
```

また、引数の数が違うタイプのオーバーロードはデフォルト引数で再現できる。

```python
C = Class {.x = Int; .y = Int}
C.
    new(x, y := 0) = Self {.x; .y}

assert C.new(0, 0) == C.new(0)
```

引数の数によって型が違うなど全く挙動が変わる関数は定義できないが、そもそも振る舞いが異なるならば別の名前を付けるべきであるというスタンスをErgは取る。

結論として、Ergがオーバーロードを禁止してサブタイピング+パラメトリック多相を採用したのは以下の理由からである。

まず、オーバーロードされた関数は定義が分散する。このため、エラーが発生した際に原因となる箇所を報告するのが難しい。
また、サブルーチンをインポートすることによって、すでに定義されたサブルーチンの挙動が変わる恐れもある。

```python
{id;} = import "foo"
...
id x: Int = x
...
id x: Ratio = x
...
id "str" # TypeError: id is not implemented for Str
# しかし、このエラーはどこから来たのだろうか?
```

次に、デフォルト引数との相性が悪い。デフォルト引数のある関数がオーバーロードされているとき、どれが優先されるかという問題がある。

```python
f x: Int = ...
f(x: Int, y := 0) = ...

f(1) # どちらが選択されるだろうか?
```

さらに、宣言との相性が悪い。
宣言`f: Num -> Num`は、どちらの定義のことを指しているのか特定できない。`Int -> Ratio`と`Ratio -> Int`は包含関係がないためである。

```python
f: Num -> Num
f(x: Int): Ratio = ...
f(x: Ratio): Int = ...
```

そして、文法の一貫性を損なう。Ergは変数の再代入を禁止するが、オーバーロードの文法は再代入のように見えてしまう。
無名関数に置換することもできない。

```python
# `f = x -> body`と同じ
f x = body

# 以下は同じ...ではない
f x: Int = x
f x: Ratio = x
```
