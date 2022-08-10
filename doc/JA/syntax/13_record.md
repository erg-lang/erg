# レコード

レコードは、キーでアクセスするDictとコンパイル時にアクセスが検査されるタプルの性質を併せ持つコレクションです。
JavaScriptをやったことがある方ならば、オブジェクトリテラル記法の(より強化された)ようなものと考えてください。

```erg
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # Error: john is not subscribable
```

`.name`, `.age`の部分を属性、`"John"`, `21`の部分を属性値と呼びます。

JavaScriptのオブジェクトリテラルとの相違点は、文字列でアクセスできない点です。すなわち、属性は単なる文字列ではありません。
これは、値へのアクセスをコンパイル時に決定するためと、辞書とレコードが別物であるためといった理由があります。つまり、`{"name": "John"}`はDict,`{name = "John"}`はレコードである。
では、辞書とレコードはどう使い分ければいいのか。
一般的にはレコードの使用を推奨する。レコードには、コンパイル時に要素が存在するかチェックされる、 __可視性(visibility)__ を指定できるなどのメリットがある。
可視性の指定は、Java言語などでみられるpublic/privateの指定に相当する。詳しくは[可視性](./15_visibility.md)を参照。

```erg
a = {x = 1; .y = x + 1}
a.x # AttributeError: x is private
# Hint: declare as `.x`
assert a.y == 2
```

上の例はJavaScriptに習熟している人間からすると奇妙かもしれないが、単に`x`と宣言すると外部からアクセスできない。`.`をつけると`.`でアクセスできるというわけである。

属性に対する明示的な型指定もできる。

```erg
anonymous = {
    .name: Option! Str = !None
    .age = 20
}
anonymous.name.set! "John"
```

レコードはメソッドも持てる。

```erg
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

レコードに関して特筆すべき文法がある。レコードの属性値が全てクラス(構造型ではダメ)のとき、そのレコード自体が、自身の属性を要求属性とする型としてふるまうのである。
このような型をレコード型と呼ぶ。詳しくは[レコード]の項を参照してほしい。

```erg
# レコード
john = {.name = "John"}
# レコード型
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am {n.name}"
greet! john # "Hello, I am John"

print! Named.name # Str
```

## レコードの分解

レコードは以下のようにして分解することができる。

```erg
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

また、レコードは属性と同名の変数があるとき、例えば`x = x`または`x = .x`を`x`に、`.x = .x`または`.x = x`を`.x`に省略することができる。
ただし属性が一つのときはセットと区別するために`;`を付ける必要がある。

```erg
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

この構文を利用して、レコードを分解して変数に代入することができる。

```erg
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

空のレコードは`{=}`で表される。空のレコードはUnitと同じく自身のクラスそのものでもある。

```erg
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

空のレコードは空のDict`{:}`や空のセット`{}`とは異なる。特に`{}`とは意味が正反対なので注意が必要である(Pythonでは`{}`は空の辞書となっているが、Ergでは`!{:}`である)。
列挙型としての`{}`は何も要素に含まない空虚な型である。`Never`型は、これをクラス化したものである。
逆に、レコードクラスの`{=}`は要求インスタンス属性がないので、全てのオブジェクトがこれの要素になる。`Object`は、これのエイリアスである。
`Object`(のパッチ)は`.__sizeof__`などの極めて基本的な提供メソッドを持つ。

```erg
AnyPatch = Patch Structural {=}
    .__sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

注意として、`{}`, `Never`型と構造的に等価な型・クラスは他に存在できず、ユーザーが`{}`, `Class {}`を右辺に指定して型を定義するとエラーとなる。
これにより、例えば`1..10 or -10..-1`とするところを`1..10 and -10..-1`としてしまうようなミスを防ぐことができる。
また、合成の結果`Object`となるような型(`Int and Str`など)を定義すると単に`Object`とするように警告が出る。

## インスタントブロック

Ergにはもう一つインスタントブロックという構文があるが、これは単に最後に評価した値を返すだけである。属性の保持はできない。

```erg
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # SyntaxError: cannot define an attribute in an entity block
```

## データクラス

素のレコード(レコードリテラルで生成されたレコード)は、これ単体でメソッドを実装しようとすると、直接インスタンスに定義する必要がある。
これは効率が悪く、さらに属性の数が増えていくとエラー表示などが見にくくなり使いにくい。

```erg
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
john + 1
# TypeError: + is not implemented for {name = Str; age = Int; .greet! = Ref(Self).() => None; inc_age! = Ref!(Self).() => None}, Int
```

そこで、このような場合はレコードクラスを継承するとよい。このようなクラスをデータクラスと呼ぶ。
これについては[クラス](./type/04_class.md)の項で詳しく説明する。

```erg
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
john + 1
# TypeError: + is not implemented for Person, Int
```

<p align='center'>
    <a href='./12_tuple.md'>Previous</a> | <a href='./14_set.md'>Next</a>
</p>
