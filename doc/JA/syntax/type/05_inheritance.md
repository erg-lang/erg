# 継承(Inheritance)


継承を使うと、既存のクラスに機能を加えたり特化したりした新しいクラスを定義できます。
継承はトレイトにおける包摂に似ています。継承してできたクラスは、もとのクラスのサブタイプになります。

```python
NewInt = Inherit Int
NewInt.
    plus1 self = self + 1

assert NewInt.new(1).plus1() == 2
assert NewInt.new(1) + NewInt.new(1) == 2
```

新しく定義するクラスを継承可能なクラスにしたい場合は`Inheritable`デコレータを付与する必要があります。

オプション引数`additional`を指定すると追加のインスタンス属性を持つことができます。ただし値クラスの場合はインスタンス属性を追加できません。

```python
@Inheritable
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}

john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

MailAddress = Inherit Str, additional: {owner = Str} # TypeError: instance variables cannot be added to a value class
```

Ergでは例外的に`Never`型の継承はできない設計となっている。`Never`は決してインスタンスを生成できない特異なクラスであるためである。

## 列挙クラスの継承

[Or型](./13_algebraic.md)をクラス化した列挙クラスも継承ができます。この際、オプション引数`Excluding`を指定することで選択肢のどれか(`or`で複数選択可)を外せます。
なお追加はできません。選択肢を追加したクラスは、元のクラスのサブタイプとはならないからです。

```python
Number = Class Int or Float or Complex
Number.
    abs(self): Float =
        match self:
            i: Int -> i.abs().into Float
            f: Float -> f.abs()
            c: Complex -> c.abs().into Float

# matchの選択肢でc: Complexは現れ得ない
RealNumber = Inherit Number, Excluding: Complex
```

同様に、[篩型](./12_refinement.md)も指定できます。

```python
Months = Class 0..12
MonthsNot31Days = Inherit Months, Excluding: {1, 3, 5, 7, 8, 10, 12}

StrMoreThan3 = Class StrWithLen N | N >= 3
StrMoreThan4 = Inherit StrMoreThan3, Excluding: StrWithLen N | N == 3
```

## オーバーライド

元の型に新しいメソッドを追加できるところはパッチと同じですが、クラスはさらに「上書き」が可能です。
この上書きをオーバーライド(override: 上書き)といいます。オーバーライドを行うには3つの条件を満たす必要があります。
まず、オーバーライドはデフォルトではエラーとなるため`Override`デコレータを付ける必要があります。
さらに、オーバーライドによってメソッドの型を変えることはできません。元の型のサブタイプである必要があります。
そして、他のメソッドから参照されているメソッドをオーバーライドする場合、参照しているメソッドも全てオーバーライドする必要があります。

なぜこのような条件が必要なのでしょうか。それは、オーバーライドが単に一つのメソッドの挙動を変えるだけでなく、別のメソッドの挙動に影響を及ぼす可能性があるからです。

まず、1つ目の条件から解説します。この条件は「不測のオーバーライド」を防ぐためです。
つまり、たまたま派生クラス側で新しく定義したつもりだったメソッドの名前が基底クラスとかちあってしまうといったことを防ぐため、`Override`デコレータで明示する必要があるのです。

次に、2つ目の条件について考えます。これは型の整合性を保つためです。派生クラスは基底クラスのサブタイプであるため、その振る舞いも基底クラスのものと互換性がなくてはなりません。

最後に、3つ目の条件について考えます。この条件はErg特有で、他のオブジェクト指向言語ではあまり見られないものですが、これも安全のためです。これがなかったとき、どんなまずいことが起こりうるか見てみましょう。

```python
# Bad example
@Inheritable
Base! = Class {x = Int!}
Base!.
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!.
    @Override
    g! ref! self = self.f!() # InfiniteRecursionWarning: This code falls into an infinite loop
    # OverrideError: method `.g` is referenced by `.f` but not overridden
```

継承クラス`Inherited!`では、`.g!`メソッドをオーバーライドして処理を`.f!`に転送しています。しかし基底クラスの`.f!`メソッドはその処理を`.g!`に転送しているので、無限ループが発生してしまっています。`.f`は`Base!`クラスでは問題の無いメソッドでしたが、オーバーライドによって想定外の使われ方をされ、壊れてしまったのです。

なので、オーバーライドの影響を受ける可能性のあるメソッドは一般に全て書き直す必要があるわけです。Ergはこのルールを仕様に組み込んでいます。

```python
# OK
@Inheritable
Base! = Class {x = Int!}
Base!.
    f! ref! self =
        print! self::x
        self.g!()
    g! ref! self = self::x.update! x -> x + 1

Inherited! = Inherit Base!
Inherited!.
    @Override
    f! ref! self =
        print! self::x
        self::x.update! x -> x + 1
    @Override
    g! ref! self = self.f!()
```

しかし、この仕様はオーバーライドの問題を完全に解決するものではありません。コンパイラはオーバーライドで問題が修正されたか検知できないためです。
オーバーライドによる影響の修正は派生クラスを作成するプログラマの責任です。可能な限り別名のメソッドを定義するようにしましょう。

### トレイトの差し替え(のように見えるもの)

継承時にトレイトを差し替えることはできませんが、一見それを行っているようにみえる例があります。

例えば`Real`(`Add()`を実装する)のサブタイプである`Int`では`Add()`を再実装しているようにみえます。

```python
Int = Class ..., Impl := Add() and ...
```

しかし実際は`Real`の`Add()`は`Add(Real, Real)`の略で、`Int`では`Add(Int, Int)`で上書きしているだけです。
両者は別のトレイトです(`Add`は[共変](./advanced/variance.md)なので`Add(Real, Real) :> Add(Int, Int)`ではありますが)。

## 多重継承の禁止

Ergでは通常のクラス同士でIntersection(交差), Diff(除外), Complement(否定)が行えません。

```python
Int and Str # TypeError: cannot unite classes
```

このルールにより、複数のクラスを継承すること、すなわち多重継承が行えません。

```python
IntAndStr = Inherit Int and Str # SyntaxError: multiple inheritance of classes is not allowed
```

ただし、Pythonの多重継承されたクラスは使用可能です。

## 多層(多段)継承の禁止

Ergの継承は多層継承も禁止しています。すなわち、継承して作ったクラスを更に継承したクラスを定義することはできません。
ただし、`Object`を継承している(Inheritable)クラスは例外的に継承可能です。

また、この場合もPythonの多層継承されたクラスは使用可能です。

## 継承元属性の書き換え禁止

Ergでは継承元の属性を書き換えることができません。これは2つの意味があります。
1つ目は、継承元のクラス属性に対する更新操作です。再代入はもちろん、`.update!`メソッドなどによる更新もできません。

オーバーライドはより特化したメソッドで上書きする操作であるため書き換えとは異なります。オーバーライドの際も互換性のある型で置き換えなくてはなりません。

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!.
    var = !1
    inc_pub! ref! self = self.pub.update! p -> p + 1

Inherited! = Inherit Base!:
Inherited!.
    var.update! v -> v + 1
    # TypeError: can't update base class variables
    @Override
    inc_pub! ref! self = self.pub + 1
    # OverrideError: `.inc_pub!` must be subtype of `Self!.() => ()`
```

2つ目は、継承元の(可変)インスタンス属性に対する更新操作です。これも禁止されています。基底クラスのインスタンス属性は、基底クラスの用意したメソッドからのみ更新できます。
属性の可視性にかかわらず、直接更新はできません。ただし読み取りはできます。

```python
@Inheritable
Base! = Class {.pub = !Int; pri = !Int}
Base!.
    inc_pub! ref! self = self.pub.update! p -> p + 1
    inc_pri! ref! self = self::pri.update! p -> p + 1

Inherited! = Inherit Base!:
Inherited!.
    # OK
    add2_pub! ref! self =
        self.inc_pub!()
        self.inc_pub!()
    # NG, `Child` cannot touch `self.pub` and `self::pri`
    add2_pub! ref! self =
        self.pub.update! p -> p + 2
```

畢竟、Ergの継承ができることは新規属性の追加と基底クラスメソッドのオーバーライドのみといえるでしょう。

## 継承の使い所

継承は正しく使えば強力な機能である反面、クラス同士の依存関係が複雑になりやすいという欠点もあり、特に多重継承・多層継承を使用した場合はその傾向が顕著となります。依存関係の複雑化はコードのメンテナンス性を下げる恐れがあります。
Ergが多重継承、多層継承を禁止したのはこの危険性を低減するためで、クラスパッチという機能を導入したのは、継承の「機能の追加」という側面を持ちながら依存関係の煩雑化を抑えるためです。

では逆に継承を使うべきところはどこでしょうか。一つの指標は、「基底クラスの意味論的なサブタイプがほしい」場合です。
Ergはサブタイプ判定の一部を型システムが自動で判定してくれます(e.g. 0以上のIntであるところのNat)。
しかし例えば、「有効なメールアドレスを表す文字列型」をErgの型システムのみに頼って作成することは困難です。通常の文字列にバリデーションを行うべきでしょう。そして、バリデーションが通った文字列オブジェクトには何らかの「保証書」を付加したいところです。それが継承クラスへのダウンキャストに相当するわけです。`Strオブジェクト`を`ValidMailAddressStr`にダウンキャストすることは、文字列が正しいメールアドレスの形式であるか検証することと一対一対応します。

```python
ValidMailAddressStr = Inherit Str
ValidMailAddressStr.
    init s: Str =
        validate s # mail-address validation
        Self.new s

s1 = "invalid mail address"
s2 = "foo@gmail.com"
_ = ValidMailAddressStr.init s1 # panic: invalid mail address
valid = ValidMailAddressStr.init s2
valid: ValidMailAddressStr # assurance that it is in the correct email address format
```

もう一つの指標は、「記名的な多相=多態を実現したい」場合です。
例えば、以下に定義する`greet!`プロシージャは、`Named`型のオブジェクトならば何でも受け付けます。
しかし、明らかに`Dog`型オブジェクトを適用するのは間違えています。そこで引数の型を`Person`クラスにします。
こうすれば、引数として受け付けるのは`Person`オブジェクトとそれを継承したクラス、`Student`オブジェクトのみです。
この方が保守的で、不必要に多くの責任を負う必要がなくなります。

```python
Named = {name = Str; ...}
Dog = Class {name = Str; breed = Str}
Person = Class {name = Str}
Student = Inherit Person, additional: {id = Int}
structural_greet! person: Named =
    print! "Hello, my name is {person::name}."
greet! person: Person =
    print! "Hello, my name is {person::name}."

max = Dog.new {name = "Max", breed = "Labrador"}
john = Person.new {name = "John"}
alice = Student.new {name = "Alice", id = 123}

structural_greet! max # Hello, my name is Max.
structural_greet! john # Hello, my name is John.
greet! alice # Hello, my name is Alice.
greet! max # TypeError:
```

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_nst_vs_sst.md'>Next</a>
</p>
