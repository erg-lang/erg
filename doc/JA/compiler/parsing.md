# Erg言語の構文解析

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/parsing.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/parsing.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## 空白の扱い

Ergの文法において特異なのは、space-sensitive(空白による区別がある)である点である。
これは、`()`の省略による表現力の低下を補うためである。同様の文法は同じく`()`を省略可能なNimでも見られる。

```python
f +1 == f(+1)
f + 1 == `+`(f, 1)
f (1,) == f((1,))
f(1,) == f(1)
(f () -> ...) == f(() -> ...)
(f() -> ...) == (f() -> ...)
```

## 左辺値、右辺値

Ergにおいて左辺値とは`=`の左側といった単純なものではない。
実際、(非常に紛らわしいが)`=`の左側にも右辺値は存在するし、`=`の右側にも左辺値が存在する。
右辺値の中に左辺値が存在することさえある。

```python
# iは左辺値、Array(Int)と[1, 2, 3]は右辺値
i: Array(Int) = [1, 2, 3]
# `[1, 2, 3].iter().map i -> i + 1`は右辺値だが、->の左側のiは左辺値
a = [1, 2, 3].iter().map i -> i + 1
# {x = 1; y = 2}は右辺値だが、x, yは左辺値
r = {x = 1; y = 2}
```

左辺値、右辺値の正確な定義は、「評価可能ならば右辺値、そうでないならば左辺値」である。
例として`i = 1; i`というコードを考える。2つ目の`i`は評価可能であるため右辺値だが、1つ目の`i`は左辺値となる。
