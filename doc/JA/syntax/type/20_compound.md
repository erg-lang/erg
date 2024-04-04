# 複合型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/compound.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

## タプル型

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

上のタプル型は構文糖であり、`(X, Y) == Tuple [X, Y]`である。

タプルには、中の型だけでなく長さについての部分型規則が存在する。
任意のタプル`T`, `U`について、以下が成り立つ。

```erg
* T <: () (ユニット規則)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U.N == T.N => U <: T (忘却規則)
```

例えば、`(Int, Str, Bool) <: (Int, Str)`である。
ただし、これらの規則は関数型のタプル(に見える)部分には適用されない。この部分は実際タプルではないためである。

```erg
(Int, Int) -> Int !<: (Int,) -> Int
```

また、ユニット型の戻り値は無視できるが、その他のタプル型の戻り値は無視できない。

## リスト型

```erg
[], [X; 0], [X; 1], [X; 2], ..., [X; _] == [X]
```

上のリスト型は構文糖であり、`[X; N] == List X, N`である。

リストに関してもタプルと同様の部分型規則が存在する。

```erg
* T <: [] (ユニット規則)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U[N] == T[N] => U <: T (忘却規則)
```

下のようなリストは型として有効ではない。リストの要素は等質化されていることを強調するための意図的な設計である。

```erg
[Int, Str]
```

このために、各要素の詳細な情報は失われてしまう。これを保つためには篩型を使う。

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## セット型

```erg
{}, {T; _}, ...
```

上のセット型は構文糖であり、`{T; N} == Set T, N`である。

セット型は長さの情報を持つが、あまり使われない。セットでは要素の重複は排除されるが、重複の判定は一般にコンパイル時には出来ないためである(そのため、長さは確定できず、多くの場合でパラメータは消去される)。そもそもセットにおいて長さの情報はあまり意味をなさない。

`{}`は空集合であり、すべての型のサブタイプである。`{T}`とすると定数`T`のみを含む型となるので注意。

## 辞書型

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

上の形の辞書型はすべて`Dict K, V`のサブタイプである。`Dict K, V`は均質化された辞書型を意味する。
`{K: V} <: Dict K, V`であり、`{X: Y, Z: W} <: Dict X or Z, Y or W`である。

## レコード型

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

上の形のレコード型はすべて`Record`のサブタイプである。
属性が非公開のレコード型は公開のレコード型のスーパータイプである。
属性は非公開化できるが、逆はできないということである。

```erg
{.i = Int} <: {i = Int}
```

## 関数型

```erg
() -> ()
Int -> Int
(Int, Str) -> Bool
(x: Int, y: Int) -> Int
(x := Int, y := Int) -> Int
(*objs: Obj) -> Str
(Int, Ref Str!) -> Int
|T: Type|(x: T) -> T
|T: Type|(x: T := NoneType) -> T # |T: Type|(x: T := X, y: T := Y) -> T (X != Y) is invalid
```

## バインドされたメソッド型

```erg
Int.() -> Int
Int.(other: Int) -> Int
# e.g. 1.__add__: Int.(Int) -> Int
```

`C.(T) -> U`の型は`T -> U`のサブタイプである。実際両者はほとんど変わらないが、`C.(T) -> U`は`C`をレシーバ型とするメソッドの型であり、`__self__`という属性からレシーバにアクセスできる。

<p align='center'>
    <a href='./19_bound.md'>Previous</a> | Next
</p>
