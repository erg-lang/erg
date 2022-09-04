# Spread assignment (展開代入)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/28_spread_syntax.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/28_spread_syntax.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

分解代入において、変数の前に`...`を置くと残りの要素を全てその変数に展開できます。これを展開代入と呼びます。

```erg
[x, ...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## Extract assignment (抽出代入)

`...`のあとに何も書かない場合、残りの要素は無視して代入されます。このタイプの展開代入を特に抽出代入と呼びます。
抽出代入は、モジュールやレコード内にある特定の属性をローカルに持ってくる際に便利な構文です。

```erg
{sin; cos; tan; ..} = import "math"
```

このようにすると、以降はローカルで`sin, cos, tan`が使用できます。

レコードでも同じようにできます。

```erg
record = {x = 1; y = 2}
{x; y; ...} = record
```

全て展開したい場合は`{*} = record`とします。OCamlなどでいう`open`です。

```erg
record = {x = 1; y = 2}
{*} = record
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>Previous</a> | <a href='./29_decorator.md'>Next</a>
</p>
