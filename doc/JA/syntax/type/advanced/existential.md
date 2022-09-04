# 存在型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/existential.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/existential.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

∀に対応する全称型があるならば、∃に対応する存在型があると考えるのが自然です。
存在型は難しいものではありません。そうと意識していないだけで、既にあなたは存在型を知っています。

```python
T: Trait
f x: T = ...
```

上のトレイト`T`は存在型として使われています。
対して下の場合の`T`はトレイトでしかなく、`X`は全称型です。

```python
f|X <: T| x: X = ...
```

実際、存在型は全称型に置き換えられます。ではなぜ存在型などというものが存在するのでしょうか。
まず、上で見たように存在型は型変数を伴わないので、型指定をシンプルにできます。
また、型変数を除去できるので全称型ならランク2を超えてしまうような型も構成できます。

```python
show_map f: (|T| T -> T), arr: [Show; _] =
    arr.map x ->
        y = f x
        log y
        y
```

しかし、見ればわかるように存在型は元の型を忘却・拡大してしまうので、戻り値の型を広げたくない場合などは全称型を使う必要があります。
逆に、引数として受け取るだけで戻り値に関係のない型は存在型で記述して構いません。

```python
# id(1): Intになって欲しい
id|T|(x: T): T = x
# |S <: Show|(s: S) -> ()は冗長
show(s: Show): () = log s
```

ちなみに、クラスは存在型とは呼びません。予めその要素となるオブジェクトが定められているためです。
存在型はあるトレイトを満たすすべての型という意味で、実際にどのような型が代入されるか知るところではないのです。
