# 型変数(Type Variable)、量化型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/15_quantified.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/15_quantified.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

型変数はサブルーチン引数の型指定などに使用する変数で、その型が任意である(単相化しない)ことを示します。
まず、型変数を導入するモチベーションとして、入力をそのまま返す`id`関数について考えましょう。

```erg
id x: Int = x
```

入力をそのまま返す`id`関数が`Int`型に対して定義されていますが、この関数は明らかに任意の型に対して定義できます。
最大のクラスを表す`Object`を使用してみましょう。

```erg
id x: Object = x

i = id 1
s = id "foo"
b = id True
```

確かに任意の型を受け付けるようになりましたが、1つ問題があります。戻り値の型が`Object`に拡大されてしまうのです。
入力が`Int`型なら`Int`型、`Str`型なら`Str`型が返るようになっていてほしいですね。

```erg
print! id 1 # <Object object>
id(1) + 1 # TypeError: cannot add `Object` and `Int`
```

入力の型と戻り値の型が同じであるようにするには、 __型変数__ を使います。
型変数は`||`(型変数リスト)中で宣言します。

```erg
id|T: Type| x: T = x
assert id(1) == 1
assert id("foo") == "foo"
assert id(True) == True
```

これを関数の __全称量化(全称化)__ と呼びます。細かい違いはありますが、他言語でジェネリクスと呼ばれる機能に相当します。そして全称量化された関数を __多相関数__ と呼びます。
多相関数の定義は、全ての型に対して同じ形の関数を定義するようなものです(Ergはオーバーロードを禁止しているので、下のコードは実際には書けません)。

```erg
id|T: Type| x: T = x
# pseudo code
# ==
id x: Int = x
id x: Str = x
id x: Bool = x
id x: Ratio = x
id x: NoneType = x
...
```

また、型変数`T`は型指定で使用されているため、`Type`型と推論できます。なので、`|T: Type|`は単に`|T|`に省略できます。
また、`|T, N| foo: [T; N]`など型オブジェクト以外の場合でも推論できる(`T: Type, N: Nat`)ならば省略できます。

また、任意の型では大きすぎる場合、制約を与えることも出来ます。
制約を与えることにはメリットもあり、例えばサブタイプ指定をすると、特定のメソッドを使えるようになります。

```erg
# T <: Add
# => TはAddのサブクラス
# => 加算ができる
add|T <: Add| l: T, r: T = l + r
```

この例では、`T`は`Add`型のサブクラスであると要求され、実際に代入される`l`と`r`の型は同じでなくてはなりません。
この場合、`T`を満たすのは`Int`や`Ratio`などです。`Int`と`Str`の加算などは定義されていないので弾かれるわけです。

このような型付けもできます。

```erg
f|
    Y, Z: Type
    X <: Add Y, O1
    O1 <: Add Z, O2
    O2 <: Add X, _
| x: X, y: Y, z: Z  =
    x + y + z + x
```

注釈リストが長くなる場合は、事前宣言するとよいでしょう。

```erg
f: |Y, Z: Type, X <: Add(Y, O1), O1 <: Add(Z, O2), O2 <: Add(X, O3)| (X, Y, Z) -> O3
f|X, Y, Z| x: X, y: Y, z: Z  =
    x + y + z + x
```

ジェネリクスを持つ多くの言語と違い、宣言した型変数はすべて、仮引数リスト内(`x: X, y: Y, z: Z`の部分)か他の型変数の引数内かで使用されていなければなりません。
これは、型変数はすべて実引数から推論可能であるというErgの言語設計からの要求です。
なので、戻り値の型など推論ができない情報は、実引数から渡します。Ergは型を実引数から渡すことができるのです。

```erg
Iterator T = Trait {
    # 戻り値の型を引数から渡している
    # .collect: |K: Type -> Type| Self(T).({K}) -> K(T)
    .collect(self(T), K: Type -> Type): K(T) = ...
    ...
}

it = [1, 2, 3].iter().map i -> i + 1
it.collect(Array) # [2, 3, 4]
```

型変数が宣言できるのは`||`の間のみである。ただし、宣言した後はスコープを抜けるまで任意の場所で使用できる。

```erg
f|X|(x: X): () =
    y: X = x.clone()
    log X.__name__
    log X

f 1
# Int
# <class Int>
```

以下のようにして、使用時に明示的に単相化もできます。

```erg
f: Int -> Int = id|Int|
```

その場合、実引数の型よりも指定された型の方が優先されます(合致していないと実引数の型が間違っているという型エラーになる)。
すなわち、実際に渡されたオブジェクトが指定された型に変換可能ならば変換され、そうでなければコンパイルエラーとなります。

```erg
assert id(1) == 1
assert id|Int|(1) in Int
assert id|Ratio|(1) in Ratio
# キーワード引数も使える
assert id|T: Int|(1) == 1
id|Int|("str") # TypeError: id|Int| is type `Int -> Int` but got Str
```

この文法が内包表記とバッティングする際は`()`で囲む必要があります。

```erg
# {id|Int| x | x <- 1..10}だと{id | ...}だと解釈される
{(id|Int| x) | x <- 1..10}
```

既に存在する型と同名の型変数は宣言出来ません。これは、型変数がすべて定数であるためです。

```erg
I: Type
# ↓ invalid type variable, already exists
f|I: Type| ... = ...
```

## メソッド定義における型引数

左辺における型引数はデフォルトで束縛型変数として扱われます。

```erg
K(T: Type, N: Nat) = ...
K(T, N).
    foo(x) = ...
```

別の型変数名を使用すると警告が出ます。

```erg
K(T: Type, N: Nat) = ...
K(U, M). # Warning: K's type variable names are 'T' and 'N'
    foo(x) = ...
```

定数は定義以降すべての名前空間で同一なので、当然型変数名にも使用できません。

```erg
N = 1
K(N: Nat) = ... # NameError: N is already defined

L(M: Nat) = ...
# M == N == 1のときのみ定義される
L(N).
    foo(self, x) = ...
# 任意のM: Natに対して定義される
L(M).
    .bar(self, x) = ...
```

型引数ごとに多重定義することはできませんが、型引数を代入していない依存型(非原始カインド)と代入した依存型(原始カインド)は関係がないので同名のメソッドを定義できます。

```erg
K(I: Int) = ...
K.
    # Kは真の型(原子カインド)ではないので、メソッドを定義できない
    # これはメソッドではない(スタティックメソッドに近い)
    foo(x) = ...
K(0).
    foo(self, x): Nat = ...
```

## 全称型

前章で定義した`id`関数は任意の型になれる関数です。では、「`id`関数自体の型」は何なのでしょうか？

```erg
print! classof(id) # |T: Type| T -> T
```

`|T: Type| T -> T`という型が得られました。これは __閉じた全称量化型/全称型(closed universal quantified type/universal type)__ と呼ばれるもので、MLでは`['a. ...]`、Haskellでは`forall t. ...`という形式で提供される型に相当します。なぜ「閉じた」という形容詞がつくのかは後述します。

閉じた全称型には制約があり、全称化できる、すなわち左の節に置けるのはサブルーチン型のみです。しかしこれで十分です。Ergではサブルーチンがもっとも基本的な制御構造ですから、「任意のXを扱いたい」というとき、すなわち「任意のXを扱えるサブルーチンがほしい」という意味になります。なので、全称型は多相関数型と同じ意味になります。以降は基本的に、この種の型を多相関数型と呼ぶことにします。

無名関数と同じく、多相関数型には型変数名の任意性がありますが、これらはすべて同値となります。

```erg
assert (|T: Type| T -> T) == (|U: Type| U -> U)
```

ラムダ計算でいうところのα同値であるときに等号が成立します。型上の演算にはいくつかの制約があるので、同値性の判定は(停止性を考えなければ)常に可能です。

## 多相関数型の部分型付け

多相関数型は、任意の関数型になれます。これは、任意の関数型と部分型関係があるということです。この関係について詳しくみていきましょう。

`OpenFn T: Type = T -> T`のような「型変数が左辺で定義され、右辺で使用されている型」を __開いた全称型(open universal type)__ と呼びます。
対して`ClosedFn = |T: Type| T -> T`など「型変数が右辺で定義・使用されている型」を __閉じた全称型(closed universal type)__ と呼びます。

開いた全称型は、同形な全ての「真の型」のスーパータイプになります。対して、閉じた全称型は、同形な全ての「真の型」のサブタイプになります。

```erg
(|T: Type| T -> T) < (Int -> Int) < (T -> T)
```

閉じている方が小さい/開いている方が大きい、と覚えるとよいでしょう。
しかし、どうしてそうなるのでしょうか。理解を深めるため、それぞれのインスタンスを考えてみます。

```erg
# id: |T: Type| T -> T
id|T|(x: T): T = x

# iid: Int -> Int
iid(x: Int): Int = x

# 任意の関数をそのまま返す
id_arbitrary_fn|T|(f1: T -> T): (T -> T) = f
# id_arbitrary_fn(id) == id
# id_arbitrary_fn(iid) == iid

# 多相関数をそのまま返す
id_poly_fn(f2: (|T| T -> T)): (|T| T -> T) = f
# id_poly_fn(id) == id
id_poly_fn(iid) # TypeError

# Int型関数をそのまま返す
id_int_fn(f3: Int -> Int): (Int -> Int) = f
# id_int_fn(id) == id|Int|
# id_int_fn(iid) == iid
```

`|T: Type| T -> T`型である`id`は`Int -> Int`型のパラメータ`f3`に代入できているため、`(|T| T -> T) < (Int -> Int)`と考えることができそうです。
その逆、`Int -> Int`型である`iid`は`(|T| T -> T)`型のパラメータ`f2`に代入できていませんが、`T -> T`型のパラメータ`f1`に代入できているため、`(Int -> Int) < (T -> T)`です。
よって、確かに`(|T| T -> T) < (Int -> Int) < (T -> T)`となっています。

## 全称型と依存型

依存型と全称型(多相関数型)はどんな関係があり、何が違うのでしょうか。
依存型は引数を取る型であり、全称型は(全称化するサブルーチンの)引数に任意性を与える型だと言えます。

重要なのは、閉じた全称型自体には型引数が存在しないというところです。例えば、多相関数型`|T| T -> T`は多相関数 __だけ__ を取る型であり、その定義は閉じています。その型引数`T`を使ったメソッド等の定義はできません。

Ergでは型自体も値であるため、引数を取る型、例えば関数型なども須らく依存型になります。つまり、多相関数型は全称型でかつ依存型でもあるといえます。

```erg
PolyFn = Patch(|T| T -> T)
PolyFn.
    type self = T # NameError: cannot find 'T'
DepFn T = Patch(T -> T)
DepFn.
    type self =
        log "by DepFn"
        T

assert (Int -> Int).type() == Int # by DepFn
assert DepFn(Int).type() == Int # by DepFn
```
