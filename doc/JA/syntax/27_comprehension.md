# Comprehension(内包表記)

`[expr | (name <- iterable)+ (predicate)*]`で配列、
`{expr | (name <- iterable)+ (predicate)*}`でセット、
`{key: value | (name <- iterable)+ (predicate)*}`でDictが作れる。

`|`で区切られた節のうち最初の部分をレイアウト節(配置節)といい、2番目の部分をバインド節(束縛節)、3番目の部分をガード節(条件節)という。
ガード節は省略可能であるがバインド節は省略できず、バインド節より先にガード節を置くことはできない。

e.g.

```erg
assert [i | i <- [0, 1, 2]] == [0, 1, 2]
assert [i / 2 | i <- 0..2] == [0.0, 0.5, 1.0]
assert [(i, j) | i <- 0..2; j <- 0..2; (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]
assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

Ergの内包表記はHaskellに影響を受けているが、若干の違いがある。
Haskellのリスト内包表記の場合、変数の順番は結果に違いをもたらすが、Ergでは関係がない。

```haskell
-- Haskell
[(i, j) | i <- [1..3], j <- [3..5]] == [(1,3),(1,4),(1,5),(2,3),(2,4),(2,5),(3,3),(3,4),(3,5)]
[(i, j) | j <- [3..5], i <- [1..3]] == [(1,3),(2,3),(3,3),(1,4),(2,4),(3,4),(1,5),(2,5),(3,5)]
```

```erg
# Erg
assert [(i, j) | i <- 1..<3; j <- 3..<5] == [(i, j) | j <- 3..<5; i <- 1..<3]
```

これはPythonと同じである。

```python
# Python
assert [(i, j) for i in range(1, 3) for j in range(3, 5)] == [(i, j) for j in range(3, 5) for i in range(1, 3)]
```

## 篩型

内包表記と似たものに、篩型がある。篩型は`{Name: Type | Predicate}`という形式で作られる型(列挙型)である。
篩型の場合、Nameは1つまででレイアウトは指定できず(ただしタプル型などにすれば複数の値は扱える)、Predicateはコンパイル時計算できるもの、つまり定数式でなくてはならない。

```erg
Nat = {I: Int | I >= 0}
# 述語式がandだけの場合、;で代替できる
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./26_pattern_matching.md'>Previous</a> | <a href='./28_spread_syntax.md'>Next</a>
</p>
