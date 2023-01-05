# カインド

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/kind.md%26commit_hash%3D44d7784aac3550ba97c8a1eaf20b9264b13d4134)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/kind.md&commit_hash=44d7784aac3550ba97c8a1eaf20b9264b13d4134)

Ergでは全てが型付けられている。型自体も例外ではない。「型の型」を表すのが __カインド(種)__ である。例えば`1`が`Int`に属しているように、`Int`は`Type`に属している。`Type`は最もシンプルなカインドである __原子カインド(Atomic kind)__ である。型理論的の記法では、`Type`は`*`に対応する。

カインドという概念で実用上重要なのは1項以上のカインド(多項カインド)である。1項のカインドは、例えば`Option`などがそれに属する。1項カインドは`Type -> Type`と表される[<sup id="f1">1</sup>](#1)。`Array`や`Option`などの __コンテナ__ は特に型を引数に取る多項カインドのことなのである。
`Type -> Type`という表記が示す通り、実は`Option`は`T`という型を受け取って`Option T`という型を返す関数である。ただし、この関数は通常の意味での関数ではないため、1項カインド(unary kind)と普通は呼称される。

なお、無名関数演算子である`->`自体も型を受け取って型を返す場合カインドとみることができる。

また、原子カインドでないカインドは型ではないことに注意してほしい。`-1`は数値だが`-`は数値ではないのと同じように、`Option Int`は型だが`Option`は型ではない。`Option`などは型構築子と呼ばれることもある。

```python
assert not Option in Type
assert Option in Type -> Type
```

なので、以下のようなコードはエラーになる。
Ergではメソッドを定義できるのは原子カインドのみで、メソッドの第一引数以外の場所で`self`という名前を使えない。

```python
# Kは単項の一種です
K: Type -> Type
K T = Class ...
K.
    foo x = ... # OK、これはいわゆるスタティックメソッドのようなもの
    bar self, x = ... # TypeError: cannot define a method to a non-type object
K(T).
    baz self, x = ... # OK
```

2項以上のカインドの例としては`{T: U}`(: `(Type, Type) -> Type`), `(T, U, V)`(: `(Type, Type, Type) -> Type`), ...などが挙げられる。

0項のカインド`() -> Type`も存在する。これは型理論的には原子カインドと同一視されることもあるが、Ergでは区別される。例としては`Class`などがある。

```python
Nil = Class()
```

## カインドの包含関係

多項カインド間にも部分型関係、もとい部分カインド関係があります。

```python
K T = ...
L = Inherit K
L <: K
```

すなわち、任意の`T`に対し`L T <: K T`ならば`L <: K`であり、その逆も成り立ちます。

```python
∀T. L T <: K T <=> L <: K
```

## 高階カインド

高階カインド(higher-order kind)というものもある。これは高階関数と同じコンセプトのカインドで、カインド自体を受け取るカインドである。`(Type -> Type) -> Type`などが高階カインドである。高階カインドに属するオブジェクトを定義してみよう。

```python
IntContainerOf K: Type -> Type = K Int
assert IntContainerOf Option == Option Int
assert IntContainerOf Result == Result Int
assert IntContainerOf in (Type -> Type) -> Type
```

多項カインドの束縛変数はK, L, ...などと表されるのが通例である(KはKindのK)。

## セットカインド

型理論において、レコードという概念がある。これはErgのレコードとほぼ同じものである[<sup id="f2">2</sup>](#2)。

```python
# これは`レコード`であり、型理論でいうところの`レコード`に相当するものである
{x = 1; y = 2}
```

レコードの値が全て型であるとき、それはレコード型といって型の一種であった。

```python
assert {x = 1; y = 2} in {x = Int; y = Int}
```

レコード型はレコードを型付けする。察しの良い方は、レコード型を型付けする「レコードカインド」があるはずだと考えたかもしれない。実際に、それは存在する。

```python
log Typeof {x = Int; y = Int} # {{x = Int; y = Int}}
```

`{{x = Int; y = Int}}`のような型がレコードカインドである。これは特別な記法ではない。単に、`{x = Int; y = Int}`のみを要素に持つ列挙型である。

```python
Point = {x = Int; y = Int}
Pointy = {Point}
```

レコードカインドの重要な特性は、`T: |T|`であり、`U <: T`であるとき、`U: |T|`であるという点にある。
これは列挙型が実際には篩型の糖衣構文であることからもわかる。

```python
# 通常のオブジェクトでは{c} == {X: T | X == c}だが、
# 型の場合等号が定義されない場合があるので|T| == {X | X <: T}となる
{Point} == {P | P <: Point}
```

型制約中の`U <: T`は、実は`U: |T|`の糖衣構文である。
このような型のセットであるカインドは一般にセットカインドと呼ばれる。セットカインドはIteratorパターンでも現れる。

```python
Iterable T = Trait {
    .Iterator = {Iterator}
    .iter = (self: Self) -> Self.Iterator T
}
```

## 多項カインドの型推論

```python
Container K: Type -> Type, T: Type = Patch K(T, T)
Container(K).
    f self = ...
Option T: Type = Patch T or NoneType
Option(T).
    f self = ...
Fn T: Type = Patch T -> T
Fn(T).
    f self = ...
Fn2 T, U: Type = Patch T -> U
Fn2(T, U).
    f self = ...

(Int -> Int).f() # どちらが選択されるだろうか?
```

上の例で、メソッド`f`はどのパッチが選ばれるのだろうか。
素朴に考えて`Fn T`が選ばれるように思われるが、`Fn2 T, U`もあり得るし、`Option T`は`T`そのままを含むので任意の型が該当し、`Container K, T`も``` `->`(Int, Int)```すなわち```Container(`->`, Int)```として`Int -> Int`にマッチする。なので、上の4つのパッチすべてが選択肢としてありえる。

この場合、以下の優先基準に従ってパッチが選択される。

* 任意の`K(T)`(e.g. `T or NoneType`)は`Type`よりも`Type -> Type`に優先的にマッチする。
* 任意の`K(T, U)`(e.g. `T -> U`)は`Type`よりも`(Type, Type) -> Type`に優先的にマッチする。
* 3項以上のカインドについても同様の基準が適用される。
* 置換する型変数が少なく済むものが選択される。例えば`Int -> Int`は`K(T, T)`(置換する型変数: K, T)や`T -> U`(置換する型変数: T, U)よりも`T -> T`(置換する型変数: T)が優先的にマッチする。
* 置換数も同じ場合は選択不能としてエラー。

---

<span id="1" style="font-size:x-small"><sup>1</sup> 型理論の記法では`*=>*` [↩](#f1)</span>

<span id="2" style="font-size:x-small"><sup>2</sup> 可視性などの微妙な違いはある。[↩](#f2)</span>
