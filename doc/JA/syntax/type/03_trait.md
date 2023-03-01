# トレイト

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/03_trait.md%26commit_hash%3Dbaf9e9597fbe528ed07a354a2b145e42ceef9e42)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/03_trait.md&commit_hash=baf9e9597fbe528ed07a354a2b145e42ceef9e42)

トレイトは、レコード型に型属性の要求を追加した記名型です。
Pythonでいう抽象基底クラス(Abstract Base Class, ABC)に類似しますが、代数的演算を行えるという特徴があります。

トレイトは別々のクラスを同一視したい場合などに使います。標準で定義されているトレイトの例には`Eq`や`Add`などがあります。
`Eq`は`==`を実装することを要求します。`Add`は`+`(中置)を実装することを要求します。

これらを実装したクラスは全てトレイトのサブタイプとして(部分的に)同一視できるわけです。

例として、ベクトルのノルム(長さ)を計算する`Norm`トレイトを定義してみましょう。

```python
Norm = Trait {.norm = (self: Self) -> Int}
```

トレイトは宣言ができるのみで実装を持てないことに注意してください。
トレイトは以下のようにしてクラスに「実装」することができます。

```python
Point2D = Class {.x = Int; .y = Int}
Point2D|<: Norm|.
    norm self = self.x**2 + self.y**2

Point3D = Class {.x = Int; .y = Int; .z = Int}
Point3D|<: Norm|.
    norm self = self.x**2 + self.y**2 + self.z**2
```

`Point2D`と`Point3D`は全く別の型ですが、`Norm`を実装したので、`.norm`メソッドを持つ型として同一視出来ます。

```python
norm x: Norm = x.norm()

assert norm(Point2D.new({x = 1; y = 2})) == 5
assert norm(Point3D.new({x = 1; y = 2; z = 3})) == 14
```

トレイトの実装では、要求属性を実装していないとエラーになります。実装していても型が合わない場合はやはりエラーになります。

```python,compile_fail
Point3D = Class {.x = Int; .y = Int; .z = Int}

Point3D|<: Norm|.
    foo self = 1
```

トレイトのうま味の一つは、後述するPatchでメソッドを自動定義できるという点です。

```python
@Attach NotEqual
Eq = Trait {.`==` = (self: Self, other: Self) -> Bool}

NotEq = Patch Eq
NotEq.
    `!=` self, other = not self.`==` other
```

`NotEq`パッチにより、`Eq`を実装する全てのクラスは自動で`!=`も実装することになります。

## トレイト上の演算

トレイトは構造型と同じく合成、置換、排除などの演算を適用できます(e.g. `T and U`)。このようにしてできたトレイトをインスタントトレイトと呼びます。

```python
T = Trait {.x = Int}
U = Trait {.y = Int}
V = Trait {.x = Int; y: Int}
assert Structural(T and U) == Structural V
assert Structural(V not U) == Structural T
W = Trait {.x = Ratio}
assert Structural(W) !=  Structural(T)
assert Structural(W) == Structural(T.replace {.x = Ratio})
```

トレイトは型でもあるので、通常の型指定にも使えます。

```python
points: [Norm; 2] = [Point2D::new(1, 2), Point2D::new(3, 4)]
assert points.iter().map(x -> x.norm()).collect(Array) == [5, 25]
```

## トレイトの包摂

関数`Subsume`によって、あるトレイトを上位型として含むトレイトを定義できます。これをトレイトの __包摂(Subsumption)__ と呼びます。
下の例でいうと、`BinAddSub`は`BinAdd`と`BinSub`を包摂しています。
これはクラスにおける継承(Inheritance)に対応しますが、継承と違い複数の基底型を`and`で合成して指定できます。`not`によって一部を除外したトレイトでもOKです。

```python
Add R = Trait {
    .Output = Type
    .`_+_` = (self: Self, R) -> Self.Output
}
ClosedAdd = Subsume Add(Self)
Sub R = Trait {
    .Output = Type
    .`_-_` = (self: Self, R) -> Self.Output
}
ClosedSub = Subsume Sub(Self)
ClosedAddSub = Subsume ClosedAdd and ClosedSub
```

## 依存トレイト

トレイトは引数を取ることができます。これは依存型と同じです。

```python
Mapper T: Type = Trait {
    .MapIter = {Iterator}
    .map = (self: Self, T -> U) -> Self.MapIter U
}

# ArrayIterator <: Mapper
# ArrayIterator.MapIter == ArrayMapper
# [1, 2, 3].iter(): ArrayIterator Int
# [1, 2, 3].iter().map(x -> "{x}"): ArrayMapper Str
assert [1, 2, 3].iter().map(x -> "\{x}").collect(Array) == ["1", "2", "3"]
```

## トレイトにおけるオーバーライド

派生トレイトでは基底トレイトの型定義をオーバーライドできます。
この場合、オーバーライドするメソッドの型は、基底メソッドの型の部分型でなければなりません。

```python
# `Self.(R) -> O`は`Self.(R) -> O or Panic`の部分型
Div R, O: Type = Trait {
    .`/` = Self.(R) -> O or Panic
}
SafeDiv R, O = Subsume Div, {
    @Override
    .`/` = Self.(R) -> O
}
```

## APIの重複するトレイトの実装と解決

実際の`Add`, `Sub`, `Mul`の定義はこのようになっています。

```python
Add R = Trait {
    .Output = Type
    .`_+_` = (Self, R) -> .Output
}
Sub R = Trait {
    .Output = Type
    .`_-_` = (Self, R) -> .Output
}
Mul R = Trait {
    .Output = Type
    .`*` = (Self, R) -> .Output
}
```

`.Output`という変数の名前が重複しています。これら複数のトレイトを同時に実装したい場合、以下のように指定します。

```python
P = Class {.x = Int; .y = Int}
# P|Self <: Add(P)|はP|<: Add(P)|に省略できる
P|Self <: Add(P)|.
    Output = P
    `_+_` self, other = P.new {.x = self.x + other.x; .y = self.y + other.y}
P|Self <: Mul(Int)|.
    Output = P
    `*` self, other = P.new {.x = self.x * other; .y = self.y * other}
```

このようにして実装した重複のあるAPIは、使用時は殆どの場合型推論されますが、`||`で明示的に型指定することで解決もできます。

```python
print! P.Output # TypeError: ambiguous type resolution
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## トレイトの特殊化実装

トレイトは特殊化実装を行うことができます。

実装の探索はトレイトの変性に従って行われます。`Add`の変性は共変なので、`Add(Nat) <: Add(Int)`であり、`Add(Nat)`の実装が可能な限り優先されます。

```python
Natural = Class { .inner = Nat }
Natural|Self <: Add(Int)|.
    Output = Int
    `_+_` self, other = self.inner + other
Natural|Self <: Add(Nat)|.
    Output = Nat
    `_+_` self, other = self.inner + other

n = Natural.new {.inner = 1}
_: Nat = n + 1
_: Int = n + -1
```

特殊化は、ベースとなる挙動を上書きしないことが前提となります。
よって、属性の型が特殊化によって部分型に縮小していない場合はエラーとなります。

```python,compile_fail
Natural|Self <: Add(Int)|.
    Output = Int
    `_+_` self, other = self.inner + other
Natural|Self <: Add(Nat)|.
    Output = Str # TypeError: Str !<: Int
    `_+_` self, other = "\{self.inner} + \{other}" # TypeError: (C, Str) -> Str !<: (C, Int) -> Int
```

## Appendix: Rustのトレイトとの違い

Ergのトレイトは[Schärli et al.](https://www.ptidej.net/courses/ift6251/fall06/presentations/061122/061122.doc.pdf)の提唱したトレイトに忠実です。
代数演算を行えるようにするためトレイトは実装を持てないようにして、必要ならばパッチをあてる設計にしています。

また、Rustのトレイトは現在のところ特殊化が行なえません。

<p align='center'>
    <a href='./02_basic.md'>Previous</a> | <a href='./04_class.md'>Next</a>
</p>
