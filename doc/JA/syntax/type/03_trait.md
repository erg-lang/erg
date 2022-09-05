# トレイト


トレイトは、レコード型に型属性の要求を追加した記名型です。
Pythonでいう抽象基底クラス(Abstract Base Class, ABC)に類似しますが、代数的演算を行えるという特徴があります。

```python
Norm = Trait {.x = Int; .y = Int; .norm = Self.() -> Int}
```

トレイトは属性とメソッドを区別しません。

トレイトは宣言ができるのみで実装を持てないことに注意してください(実装は後に述べるパッチという機能で実現します)。
トレイトは部分型指定でクラスに実装されているかチェックできます。

```python
Point2D <: Norm
Point2D = Class {.x = Int; .y = Int}
Point2D.norm self = self.x**2 + self.y**2
```

要求属性を実装していないとエラーになります。

```python
Point2D <: Norm # TypeError: Point2D is not a subtype of Norm
Point2D = Class {.x = Int; .y = Int}
```

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

展開演算子`...`によって、あるトレイトを上位型として含むトレイトを定義できます。これをトレイトの __包摂(Subsumption)__ と呼びます。
下の例でいうと、`BinAddSub`は`BinAdd`と`BinSub`を包摂しています。
これはクラスにおける継承(Inheritance)に対応しますが、継承と違い複数の基底型を`and`で合成して指定できます。`not`によって一部を除外したトレイトでもOKです。

```python
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
ClosedAdd = Subsume Add(Self)
Sub R = Trait {
    .SubO = Type
    .`_-_` = Self.(R) -> O
}
ClosedSub = Subsume Sub(Self)
ClosedAddSub = Subsume ClosedAdd and ClosedSub
```

## 構造的トレイト

トレイトは構造化できます。

```python
SAdd = Structural Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: SAdd|は省略できない
add|A <: SAdd| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

assert add(C.new(1), C.new(2)) == C.new(3)
```

記名的トレイトは単に要求メソッドを実装しただけでは使えず、実装したことを明示的に宣言する必要があります。
以下の例では明示的な実装の宣言がないため、`add`が`C`型の引数で使えません。`C = Class {i = Int}, Impl := Add`としなくてはならないのです。

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: Add|は省略できる
add|A <: Add| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

add C.new(1), C.new(2) # TypeError: C is not subclass of Add
# hint: inherit or patch 'Add'
```

構造的トレイトはこの実装の宣言がなくてもよいのですが、そのかわり型推論が効きません。使う際は型指定が必要です。

## 依存トレイト

トレイトは引数を取ることができます。これは依存型と同じです。

```python
Mapper T: Type = Trait {
    .MapIter = {Iterator}
    .map = Self(T).(T -> U) -> Self.MapIter U
}

# ArrayIterator <: Mapper
# ArrayIterator.MapIter == ArrayMapper
# [1, 2, 3].iter(): ArrayIterator Int
# [1, 2, 3].iter().map(x -> "{x}"): ArrayMapper Str
assert [1, 2, 3].iter().map(x -> "{x}").collect(Array) == ["1", "2", "3"]
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
    .`_+_` = Self.(R) -> .Output
}
Sub R = Trait {
    .Output = Type
    .`_-_` = Self.(R) -> .Output
}
Mul R = Trait {
    .Output = Type
    .`*` = Self.(R) -> .Output
}
```

`.Output`という変数の名前が重複しています。これら複数のトレイトを同時に実装したい場合、以下のように指定します。

```python
P = Class {.x = Int; .y = Int}
# P|Self <: Add(P)|はP|<: Add(P)|に省略可能
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

## Appendix: Rustのトレイトとの違い

Ergのトレイトは[Schärli et al.](https://www.ptidej.net/courses/ift6251/fall06/presentations/061122/061122.doc.pdf)の提唱したトレイトに忠実です。
代数演算を行えるようにするためトレイトは実装を持てないようにして、必要ならばパッチをあてる設計にしています。

<p align='center'>
    <a href='./02_basic.md'>Previous</a> | <a href='./04_class.md'>Next</a>
</p>
