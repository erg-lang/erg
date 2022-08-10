# 変性(variance)

Ergは多相型のサブタイピングを行えるが、一部注意しなくてはならない点がある。

まずは通常の多相型の包含関係を考える。一般に、コンテナ`K`と代入する型`A, B`があり、`A < B`のとき、`K A < K B`となる。
例えば、`Option Int < Option Object`となる。よって、`Option Object`で定義されているメソッドは、`Option Int`でも使用可能である。

典型的な多相型である`Array!(T)`型について考える。
今回は要素の数を問題にしないので`Array!(T, N)`ではないことに注意してほしい。
さて、`Array!(T)`型には`.push!`と`.pop!`というメソッドが存在し、それぞれ、要素の追加・取り出しを意味する。型はこうである。

Array.push!: Self(T).(T) => NoneType
Array.pop!: Self(T).() => T

直感的に理解できることとして、

* `s: Str`のとき`Array!(Object).push!(s)`はOK(`Str`を`Object`にアップキャストすれば良い)
* `o: Object`のとき`Array!(Str).push!(o)`はNG
* `Array!(Object).pop!().into(Str)`はNG
* `Array!(Str).pop!().into(Object)`はOK

である。これは、型システム的には

* (Self(Object).(Object) => NoneType) < (Self(Str).(Str) => NoneType)
* (Self(Str).() => Str) < (Self(Object).() => Object)

を意味する。

前者は奇妙に思えるかもしれない。`Str < Object`なのに、それを引数に取る関数は包含関係が逆転している。
型理論では、このような関係(`.push!`の型関係)を反変(contravariant)といい、その逆、`.pop!`の型関係は共変(covariant)という。
つまり、関数型は引数の型に関して反変であり、戻り値の型に関して共変である、といえる。
複雑に聞こえるが、先程見た通り実例に当てはめて考えれば合理的なルールである。
それでもいまいちピンと来ない場合は次のように考えるとよい。

Ergの設計方針に、「入力の型は大きく、出力の型は小さく」というのがある。これはまさに関数の変性から言える。
上のルールを見れば、入力型は大きい方が全体として小さい型になる。
汎用の関数は明らかに専用の関数より希少だからである。
そして出力型は小さい方が全体として小さくなる。

結果として上の方針は「関数の型を最小化せよ」と言っているのに等しい。

## 非変性

Ergにはもう一つ変性がある。それは非変性(non-variance)である。
これは組み込み型では`SharedCell! T!`などが持っている変性である。これは、`T! != U!`なる2つの型`T!, U!`に関して、例え包含関係があったとしても`SharedCell! T!`と`SharedCell! U!`間でキャストができないことを意味する。
これは、`SharedCell! T!`が共有参照であることに由来する。詳しくは[共有参照](shared.md)を参照。

## 変性指定された全称型

全称型の型変数は、その上限・下限を指定することができます。

```erg
|A <: T| K(A)
|B :> T| K(B)
```

型変数リスト内では型変数の __変性指定__ を行っています。上の変性指定において、型変数`A`は型`T`に対する任意のサブクラスであり、型変数`B`は型`T`に対する任意のスーパークラスであると宣言されています。
このとき、`T`を`A`に対する上限型、`B`に対する下限型ともいいます。

変性指定は重ねがけすることもできます。

```erg
# U < A < T
{... | A <: T; A :> U}
```

以下に変性指定を使ったコードの例を示します。

```erg
show|S <: Show| s: S = log s

Nil T = Class(Impl=Phantom T)
Cons T = Class(Nil T or List T)
List T = Class {head = T; rest = Cons T}
List(T).
    push|U <: T|(self, x: U): List T = Self.new {head = x; rest = self}
    upcast(self, U :> T): List U = self
```

## 変性指定

`List T`の例については注意が必要なので、もう少し詳しく説明します。
上のコードを理解するためには多相型の変性について知っておく必要があります。変性については[この項](./variance.md)で詳しく解説していますが、さしあたって必要となる事実は以下の3つです：

* 通常の多相型、`List T`などは`T`に対して共変(`U > T`のとき`List U > List T`)
* 関数`T -> U`は引数型`T`に対して反変(`S > T`のとき`(S -> U) < (T -> U)`)
* 関数`T -> U`は戻り値型`U`に対して共変(`U > S`のとき`(T -> U) > (T -> S)`)

例えば、`List Int`は`List Object`にアップキャスト可能、`Obj -> Obj`は`Int -> Obj`にアップキャスト可能であるということです。

ここで、メソッドの変性指定を省略した場合どうなるか考えます。

```erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    # List T can be pushed U if T > U
    push|U|(self, x: U): List T = Self.new {head = x; rest = self}
    # List T can be List U if T < U
    upcast(self, U): List U = self
```

この場合でも、Ergコンパイラは`U`の上限・下限型をよしなに推論してくれます。
ただし、Ergコンパイラはメソッドの意味を理解しないことに注意してください。コンパイラはただ変数・型変数の使われ方に従って機械的に型関係を推論・導出します。

コメントに書いてある通り、`List T`の`head`に入れられる型`U`は`T`のサブクラス(`T: Int`ならば`Nat`など)です。すなわち、`U <: T`と推論されます。この制約は`.push{U}`の引数型を変更するアップキャスト`(List(T), U) -> List(T) to (List(T), T) -> List(T)`(e.g. `List(Int).push{Object}`)を禁止します。ただし、`U <: T`という制約は関数の型の包含関係を改変しているわけではないことに注意してください。`(List(Int), Object) -> List(Int) to (List(Int), Int) -> List(Int)`である事実は変わらず、ただ`.push`メソッドにおいてはそのようなアップキャストを実行できないという意味になります。
同様に、`List T`から`List U`へのキャストは`U :> T`という制約のもとで可能なので、そのように変性指定が推論されます。この制約は、`.upcast(U)`の戻り値型を変更するアップキャスト`List(T) -> List(T) to List(T) -> List(T)`(e.g. `List(Object).upcast(Int)`)を禁止します。

では、このアップキャストを許可するようにした場合はどうなるか考えます。
変性指定を反転させてみましょう。

```erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    push|U :> T|(self, x: U): List T = Self.new {head = x; rest = self}
    upcast(self, U :> T): List U = self
# TypeWarning: `U` in the `.push` cannot take anything other than `U == T`. Replace `U` with `T`. Or you may have the wrong variance specification.
# TypeWarning: `U` in the `.upcast` cannot take anything other than `U == T`. Replace `U` with `T`. Or you may have the wrong variance specification.
```

`U <: T`という制約と`U :> T`という変性指定の両方を充足するのは`U == T`のときだけです。なので、この指定にはほとんど意味がありません。
実際は「`U == T`であるようなアップキャスト」=「`U`の箇所については変えないアップキャスト」のみが許可されています。

## Appendix: ユーザー定義型の変性

ユーザー定義型の変性は、デフォルトでは非変である。しかし、`Inputs/Outputs`というマーカートレイトで変性を指定することもできる。
`Inputs(T)`と指定すると、その型は`T`に関して反変となる。
`Outputs(T)`と指定すると、その型は`T`に関して共変となる。

```erg
K T = Class(...)
assert not K(Str) <= K(Object)
assert not K(Str) >= K(Object)

InputStream T = Class ..., Impl: Inputs(T)
# Objectを受け入れるストリームは、Strを受け入れるともみなせる
assert InputStream(Str) > InputStream(Object)

OutputStream T = Class ..., Impl: Outputs(T)
# Strを出力するストリームは、Objectを出力するともみなせる
assert OutputStream(Str) < OutputStream(Object)
```
