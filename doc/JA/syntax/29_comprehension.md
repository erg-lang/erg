# 内包表記

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_comprehension.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_comprehension.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

`[expr | (name <- iterable)+ | (predicate)*]`でリスト、
`{expr | (name <- iterable)+ | (predicate)*}`でセット、
`{key: value | (name <- iterable)+ | (predicate)*}`でDictが作れます。

`|`で区切られた節のうち最初の部分をレイアウト節(配置節)といい、2番目の部分をバインド節(束縛節)、3番目の部分をガード節(条件節)といいます。
ガード節かレイアウト節のどちらかは省略可能ですがバインド節は省略できず、またこれらの順番を入れ替えることはできません。

内包表記の例

```python
# レイアウト節はi
# バインド節はi <- [0, 1, 2]
assert [i | i <- [0, 1, 2]] == [0, 1, 2]

# フィルタリングだけしたい場合は、レイアウト節を省略できる
# これは[0, 1, 2].iter().filter(i -> i % 2 == 0).into_array()と同じ
assert [i <- [0, 1, 2] | i % 2 == 0] == [0, 2]

# レイアウト節はi / 2
# バインド節はi <- 0..2
assert [i / 2 | i <- 0..2] == [0.0, 0.5, 1.0]

# レイアウト節は(i, j)
# バインド節はi <- 0..2, j <- 0..2
# ガード節は(i + j) % 2 == 0
assert [(i, j) | i <- 0..2; j <- 0..2 | (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]

assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

## 篩型

内包表記と似たものに、篩型があります。篩型は`{Name: Type | Predicate}`という形式で作られる型(列挙型)です。
篩型の場合、Nameは1つまででレイアウトは指定できず(ただしタプル型などにすれば複数の値は扱えます)、Predicateはコンパイル時計算できるもの、つまり定数式のみが指定できます。

```python
Nat = {I: Int | I >= 0}
# 述語式がandだけの場合、;で代替できる
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./28_pattern_matching.md'>Previous</a> | <a href='./30_spread_syntax.md'>Next</a>
</p>
