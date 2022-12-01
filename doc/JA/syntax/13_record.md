# レコード

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/13_record.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/13_record.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

レコードは、キーでアクセスするDictとコンパイル時にアクセスが検査されるタプルの性質を併せ持つコレクションです。
JavaScriptをやったことがある方ならば、オブジェクトリテラル記法の(より強化された)ようなものと考えてください。

```python
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # Error: john is not subscribable
```

`.name`, `.age`の部分を属性、`"John"`, `21`の部分を属性値と呼びます。

JavaScriptのオブジェクトリテラルとの相違点は、文字列でアクセスできない点です。すなわち、属性は単なる文字列ではありません。
これは、値へのアクセスをコンパイル時に決定するためと、辞書とレコードが別物であるためといった理由があります。つまり、`{"name": "John"}`はDict,`{name = "John"}`はレコードです。
では、辞書とレコードはどう使い分ければいいのでしょうか。
一般的にはレコードの使用を推奨します。レコードには、コンパイル時に要素が存在するかチェックされる、 __可視性(visibility)__ を指定できるなどのメリットがあります。
可視性の指定は、Java言語などでみられるpublic/privateの指定に相当します。詳しくは[可視性](./19_visibility.md)を参照してください。

```python
a = {x = 1; .y = x + 1}
a.x # AttributeError: x is private
# Hint: declare as `.x`.
assert a.y == 2
```

上の例はJavaScriptに習熟している人間からすると奇妙かもしれませんが、単に`x`と宣言すると外部からアクセスできず、`.`をつけると`.`でアクセスできるというわけです。

属性に対する明示的な型指定もできます。

```python
anonymous = {
    .name: Option! Str = !None
    .age = 20
}
anonymous.name.set! "John"
```

レコードはメソッドも持てます。

```python
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

レコードに関して特筆すべき文法があります。レコードの属性値が全てクラス(構造型ではダメです)のとき、そのレコード自体が、自身の属性を要求属性とする型としてふるまいます。
このような型をレコード型と呼びます。詳しくは[レコード]の項を参照してください。

```python
# レコード
john = {.name = "John"}
# レコード型
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am \{n.name}"
greet! john # "Hello, I am John"

print! Named.name # Str
```

## レコードの分解

レコードは以下のようにして分解できます。

```python
record = {x = 1; y = 2}
{x = a; y = b} = record
assert a == 1
assert b == 2

point = {x = 2; y = 3; z = 4}
match point:
    {x = 0; y = 0; z = 0} -> "origin"
    {x = _; y = 0; z = 0} -> "on the x axis"
    {x = 0; ...} -> "x = 0"
    {x = x; y = y; z = z} -> "({x}, {y}, {z})"
```

また、レコードは属性と同名の変数があるとき、例えば`x = x`または`x = .x`を`x`に、`.x = .x`または`.x = x`を`.x`に省略できます。
ただし、属性が一つのときはセットと区別するために`;`を付ける必要があります。

```python
x = 1
y = 2
xy = {x; y}
a = 1
b = 2
ab = {.a; .b}
assert ab.a == 1
assert ab.b == 2

record = {x;}
tuple = {x}
assert tuple.1 == 1
```

この構文を利用して、レコードを分解して変数に代入できます。

```python
# same as `{x = x; y = y} = xy`
{x; y} = xy
assert x == 1
assert y == 2
# same as `{.a = a; .b = b} = ab`
{a; b} = ab
assert a == 1
assert b == 2
```

## 空レコード

空のレコードは`{=}`で表されます。空のレコードはUnitと同じく、自身のクラスそのものでもあります。

```python
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

空のレコードは空のDict`{:}`や空のセット`{}`とは異なります。特に`{}`とは意味が正反対なので注意が必要です(Pythonでは`{}`は空の辞書となっているが、Ergでは`!{:}`です)。
列挙型としての`{}`は何も要素に含まない空虚な型です。`Never`型は、これをクラス化したものです。
逆に、レコードクラスの`{=}`は要求インスタンス属性がないので、全てのオブジェクトがこれの要素になります。`Object`は、これのエイリアスです。
`Object`(のパッチ)は`.__sizeof__`などの極めて基本的な提供メソッドを持ちます。

```python
AnyPatch = Patch Structural {=}
    .__sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

注意として、`{}`, `Never`型と構造的に等価な型・クラスは他に存在できず、ユーザーが`{}`, `Class {}`を右辺に指定して型を定義するとエラーとなります。
これにより、例えば`1..10 or -10..-1`とするところを`1..10 and -10..-1`としてしまうようなミスを防げます。
また、合成の結果`Object`となるような型(`Int and Str`など)を定義すると、単に`Object`とするように警告が出ます。

## インスタントブロック

Ergにはもう一つインスタントブロックという構文がありますが、これは単に最後に評価した値を返すだけです。属性の保持はできません。

```python
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # SyntaxError: cannot define an attribute in an entity block
```

## データクラス

素のレコード(レコードリテラルで生成されたレコード)は、これ単体でメソッドを実装しようとすると、直接インスタンスに定義する必要があります。
これは効率が悪く、さらに属性の数が増えていくとエラー表示などが見にくくなり使いにくいです。

```python
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is \{self::name} and I am \{self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
john + 1
# TypeError: + is not implemented for {name = Str; age = Int; .greet! = Ref(Self).() => None; inc_age! = Ref!(Self).() => None}, Int
```

そこで、このような場合はレコードクラスを継承します。このようなクラスをデータクラスと呼びます。
これについては[クラス](./type/04_class.md)の項で詳しく説明します。

```python
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is \{self::name} and I am \{self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
john + 1
# TypeError: + is not implemented for Person, Int
```

<p align='center'>
    <a href='./12_dict.md'>Previous</a> | <a href='./14_set.md'>Next</a>
</p>
