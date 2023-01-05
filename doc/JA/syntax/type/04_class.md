# クラス

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/04_class.md%26commit_hash%3D7078f95cecc961a65befb15929af06ae2331c934)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/04_class.md&commit_hash=7078f95cecc961a65befb15929af06ae2331c934)

Ergにおけるクラスは、大まかには自身の要素(インスタンス)を生成できる型と言えます。
以下は単純なクラスの例です。

```python
Person = Class {.name = Str; .age = Nat}
# .newが定義されなかった場合、自動で`Person.new = Person::__new__`となる
Person.
    new name, age = Self::__new__ {.name = name; .age = age}

john = Person.new "John Smith", 25
print! john # <Person object>
print! classof(john) # Person
```

`Class`に与えられる型(通常はレコード)を要件型(この場合は`{.name = Str; .age = Nat}`)といいます。
インスタンスは`クラス名::__new__ {属性名 = 値; ...}`で生成できます。
`{.name = "John Smith"; .age = 25}`は単なるレコードですが、`Person.new`を通すことで`Person`インスタンスに変換されるわけです。
このようなインスタンスを生成するサブルーチンはコンストラクタと呼ばれます。
上のクラスでは、フィールド名等を省略できるように`.new`メソッドを定義しています。

以下のように改行せず定義すると文法エラーになるので注意してください。

```python,compile_fail
Person.new name, age = ... # SyntaxError: cannot define attributes directly on an object
```

> __Warning__: これは最近追加された仕様なので、以降のドキュメントでは守られていない場合があります。見つけた場合は報告してください。

## インスタンス属性、クラス属性

Pythonやその他の言語では、以下のようにブロック側でインスタンス属性を定義することが多いが、このような書き方はErgでは別の意味になるので注意が必要である。

```python
# Python
class Person:
    name: str
    age: int
```

```python
# Ergでこの書き方はクラス属性の宣言を意味する(インスタンス属性ではない)
Person = Class()
Person.
    name: Str
    age: Int
```

```python
# 上のPythonコードに対応するErgコード
Person = Class {
    .name = Str
    .age = Nat
}
```

要素属性(レコード内で定義した属性)と型属性(クラスの場合は特にインスタンス属性/クラス属性とも呼ばれる)は全くの別物である。型属性は型自体の持つ属性である。型の要素は、自らの中に目当ての属性がないときに型属性を参照する。要素属性は要素が直接持つ固有の属性である。
なぜこのような区分けがされているか。仮に全てが要素属性だと、オブジェクトを生成した際に全ての属性を複製・初期化する必要があり、非効率であるためである。
また、このように分けたほうが「この属性は共用」「この属性は別々に持つ」などの役割が明確になる。

下の例で説明する。`species`という属性は全てのインスタンスで共通なので、クラス属性とした方が自然である。だが`name`という属性は各インスタンスが個々に持っておくべきなのでインスタンス属性とすべきなのである。

```python
Person = Class {name = Str}
Person::
    species = "human"
Person.
    describe() =
        log "species: \{Person::species}"
    greet self =
        log "Hello, My name is \{self::name}."

Person.describe() # species: human
Person.greet() # TypeError: unbound method Person.greet needs an argument

john = Person.new {name = "John"}
john.describe() # species: human
john.greet() # Hello, My name is John.

alice = Person.new {name = "Alice"}
alice.describe() # species: human
alice.greet() # Hello, My name is Alice.
```

因みに、インスタンス属性と型属性で同名、同型のものが存在する場合、コンパイルエラーとなる。これは混乱を避けるためである。

```python
C = Class {.i = Int}
C.
    i = 1 # AttributeError: `.i` is already defined in instance fields
```

## Class, Type

`1`のクラスと型が違うものであることに注意してください。
`1`の生成元であるクラスは`Int`ただひとつです。オブジェクトの属するクラスは`classof(obj)`または`obj.__class__`で取得できます。
対して`1`の型は無数にあります。例としては、`{1}, {0, 1}, 0..12, Nat, Int, Num`などです。
ただし最小の型はひとつに定めることができ、この場合は`{1}`です。オブジェクトの属する型は`Typeof(obj)`で取得できます。
これはコンパイル時関数であり、その名の通りコンパイル時に計算されます。
オブジェクトからは、クラスメソッドの他にパッチメソッドも使用可能です。
Ergではクラスメソッドを追加したりはできませんが、[パッチ](./07_patch.md)で拡張可能です。

既存のクラスを継承することも出来ます([Inheritable](../29_decorator.md#inheritable)クラスの場合)。
`Inherit`は継承を意味します。左辺の型をサブクラス、右辺の`Inherit`の引数型をスーパークラスと言います。

```python
MyStr = Inherit Str
# other: StrとしておけばMyStrでもOK
MyStr.
    `-` self, other: Str = self.replace other, ""

abc = MyStr.new("abc")
# ここの比較はアップキャストが入る
assert abc - "b" == "ac"
```

Pythonと違い、定義されたErgのクラスはデフォルトで`final`(継承不可)です。
継承可能にするためには`Inheritable`デコレータをクラスに付ける必要があります。
`Str`は継承可能クラスのひとつです。

```python
MyStr = Inherit Str # OK
MyStr2 = Inherit MyStr # NG

@Inheritable
InheritableMyStr = Inherit Str
MyStr3 = Inherit InheritableMyStr # OK
```

`Inherit Obj`と`Class()`は実用上ほぼ等価です。一般的には後者を使用します。

クラスは型とは同値判定の仕組みが異なります。
型は構造に基づいて同値性が判定されます。

```python
Person = {.name = Str; .age = Nat}
Human = {.name = Str; .age = Nat}

assert Person == Human
```

クラスは同値関係が定義されていません。

```python
Person = Class {.name = Str; .age = Nat}
Human = Class {.name = Str; .age = Nat}

Person == Human # TypeError: cannot compare classes
```

## 構造型との違い

クラスは自身の要素を生成することができる型といいましたが、それだけは厳密な説明ではありません。実際はレコード型+パッチでも同じことができるからです。

```python
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    new name, age = {.name; .age}

john = Person.new("John Smith", 25)
```

クラスを使用するメリットは4つあります。
1つはコンストラクタの正当性が検査されること、2つ目はパフォーマンスが高いこと、3つ目は記名的部分型(NST)が使用できること、4つ目は継承・オーバーライドができることです。

先程レコード型+パッチでもコンストラクタ(のようなもの)が定義できることを見ましたが、これはもちろん正当なコンストラクタとは言えません。`.new`と名乗っていても全く関係のないオブジェクトを返すことができるからです。クラスの場合は、`.new`が要件を満たすオブジェクトを生成するか静的に検査されます。

~

クラスの型検査は、単にオブジェクトの`.__class__`属性を見るだけで完了します。なので、オブジェクトが型に属しているかの検査が高速です。

~

ErgではクラスでNSTを実現します。NSTの利点として、堅牢性などが挙げられます。
大規模なプログラムを書いていると、オブジェクトの構造が偶然一致することはままあります。

```python
Dog = {.name = Str; .age = Nat}
DogImpl = Patch Dog
DogImpl.
    bark = log "Yelp!"
...
Person = {.name = Str; .age = Nat}
PersonImpl = Patch Person
PersonImpl.
    greet self = log "Hello, my name is \{self.name}."

john = {.name = "John Smith"; .age = 20}
john.bark() # "Yelp!"
```

`Dog`と`Person`の構造は全く同一ですが、動物が挨拶したり人間が吠えたりできるようにするのは明らかにナンセンスです。
後者はともかく、前者は不可能なので適用できないようにする方が安全です。このような場合はクラスを使用すると良いでしょう。

```python
Dog = Class {.name = Str; .age = Nat}
Dog.
    bark = log "Yelp!"
...
Person = Class {.name = Str; .age = Nat}
Person.
    greet self = log "Hello, my name is \{self.name}."

john = Person.new {.name = "John Smith"; .age = 20}
john.bark() # TypeError: `Person` object has no method `.bark`
```

もう一つ、パッチによって追加された型属性は仮想的なもので、実装するクラスが実体として保持している訳ではないという特徴があります。
つまり、`T.x`, `T.bar`は`{i = Int}`と互換性のある型がアクセスできる(コンパイル時に結びつける)オブジェクトであり、`{i =  Int}`や`C`に定義されているわけではありません。
対してクラス属性はクラス自身が保持しています。なので、構造が同じであっても継承関係にないクラスからはアクセスできません。

```python
C = Class {i = Int}
C.
    foo self = ...
print! dir(C) # ["foo", ...]

T = Patch {i = Int}
T.
    x = 1
    bar self = ...
print! dir(T) # ["bar", "x", ...]
assert T.x == 1
assert {i = 1}.x == 1
print! T.bar # <function bar>
{i = Int}.bar # TypeError: Record({i = Int}) has no method `.bar`
C.bar # TypeError: C has no method `.bar`
print! {i = 1}.bar # <method bar>
print! C.new({i = 1}).bar # <method bar>
```

## データクラスとの違い

クラスには、レコードを要求型とする`Class`を通した通常のクラスと、レコードを継承(`Inherit`)したデータクラスがあります。
データクラスはレコードの機能を受け継いでおり、分解代入ができる、`==`や`hash`がデフォルトで実装されているなどの特徴があります。
逆に独自の同値関係やフォーマット表示を定義したい場合は通常のクラスを使用するとよいでしょう。

```python
C = Class {i = Int}
c = C.new {i = 1}
d = C.new {i = 2}
print! c # <C object>
c == d # TypeError: `==` is not implemented for `C`

D = Inherit {i = Int}
e = D::{i = 1} # e = D.new {i = 1}と同じ
f = D::{i = 2}
print! e # D(i = 1)
assert e != f
```

## Enum Class

Or型のクラスを定義しやすくするために、`Enum`が用意されています。

```python
X = Class()
Y = Class()
XorY = Enum X, Y
```

それぞれの型には`XorY.X`, `XorY.Y`のようにしてアクセスでき、コンストラクタは`X.new |> XorY.new`のようにして取得できます。

```python
x1 = XorY.new X.new()
x2 = (X.new |> XorY.new())()
x3 = (Y.new |> XorY.new())()
assert x1 == x2
assert x1 != x3
```

## 包含関係

クラスは、要件型のサブタイプです。要件型のメソッド(パッチメソッド含む)を使用できます。

```python
T = Trait {.foo = Foo}
C = Class(..., Impl: T)
C.
    foo = foo
    bar x = ...
assert C < T
assert C.foo == foo
assert not T < C
assert T.foo == Foo
```

<p align='center'>
    <a href='./03_trait.md'>Previous</a> | <a href='./05_inheritance.md'>Next</a>
</p>
