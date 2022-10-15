# 代数演算型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/13_algebraic.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/13_algebraic.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

代数演算型は、型を代数のようにみなして演算することで生成される型のことです。
代数演算型が扱う演算は、Union, Intersection, Diff, Complementなどがあります。
通常のクラスはUnionのみが行えて、他の演算は型エラーになります。

## 合併型

Union型では型について複数の可能性を与える事ができる。名前の通り、`or`演算子で生成されます。
代表的なUnionは`Option`型です。`Option`型は`T or NoneType`のpatch typeで、主に失敗するかもしれない値を表現します。

```python
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# 暗黙に`T != NoneType`となる
Option T = T or NoneType
```

## 交差型

Intersection型は型同士を`and`演算で結合して得られます。

```python
Num = Add and Sub and Mul and Eq
```

先述したように通常のクラス同士では`and`演算で結合できません。インスタンスは唯一つのクラスに属するからです。

## 除外型

Diff型は`not`演算で得られます。
英文に近い表記としては`and not`とした方が良いですが、`and`, `or`と並べて収まりが良いので`not`だけで使うのが推奨されます。

```python
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

## 否定型

Complement型は`not`演算で得られますが、これは単項演算です。`not T`型は`{=} not T`の短縮記法です。
`not T`型によるIntersectionはDiffと同等で、`not T`型によるDiffはIntersectionと同等です。
しかしこのような書き方は推奨されません。

```python
# 最も単純な非ゼロ数型の定義
NonZero = Not {0}
# 非推奨のスタイル
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## 真の代数演算型

代数演算型には、簡約可能な見かけ上の代数演算型とそれ以上簡約できない「真の代数演算型」があります。
そうではない「見かけの代数型」には、Enum型やInterval型、レコード型の`or`や`and`があります。
これらは簡約が可能なので真の代数演算型ではなく、型指定に使うとWarningが出ます。Warningを消すためには簡約化するか型定義を行うかする必要があります。

```python
assert {1, 2, 3} or {2, 3} == {1, 2, 3}
assert {1, 2, 3} and {2, 3} == {2, 3}
assert -2..-1 or 1..2 == {-2, -1, 1, 2}

i: {1, 2} or {3, 4} = 1 # TypeWarning: {1, 2} or {3, 4} can be simplified to {1, 2, 3, 4}
p: {x = Int, ...} and {y = Int; ...} = {x = 1; y = 2; z = 3}
# TypeWaring: {x = Int, ...} and {y = Int; ...} can be simplified to {x = Int; y = Int; ...}

Point1D = {x = Int; ...}
Point2D = Point1D and {y = Int; ...} # == {x = Int; y = Int; ...}
q: Point2D = {x = 1; y = 2; z = 3}
```

真の代数演算型には、`Or`型、`And`型があります。クラス同士の`or`などは`Or`型です。

```python
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff, Complement型は必ず簡約できるので真の代数演算型ではありません。
