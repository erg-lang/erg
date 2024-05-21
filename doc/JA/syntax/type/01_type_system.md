# Ergの型システム

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/01_type_system.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/01_type_system.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

以下では、Ergの型システムを概略的に説明します。詳細については他の項で解説します。

## 定義方法

Ergの特徴的な点として、(通常の)変数、関数(サブルーチン)、型(カインド)の定義にあまり大きな構文上の違いがないというところがあります。すべて、通常の変数・関数定義の文法に従って定義されます。

```python
f i: Int = i + 1
f # <function f>
f(1) # 2
f.method self = ... # SyntaxError: cannot define a method to a subroutine

T I: Int = {...}
T # <kind 'T'>
T(1) # Type T(1)
T.method self = ...
D = Class {private = Int; .public = Int}
D # <class 'D'>
o1 = {private = 1; .public = 2} # o1はどのクラスにも属さないオブジェクト
o2 = D.new {private = 1; .public = 2} # o2はDのインスタンス
o2 = D.new {.public = 2} # InitializationError: class 'D' requires attribute 'private'(: Int) but not defined
```

## 分類

Erg のオブジェクトは全て型付けされています。
最上位の型は`{=}`であり、`__repr__`, `__hash__`, `clone`などを実装します(要求メソッドではなく、これらの属性はオーバーライドもできません)。
Ergの型システムは構造的部分型(Structural subtyping, SST)を取り入れています。このシステムにより型付けされる型を構造型(Structural type)と呼びます。
構造型には大きく分けて3種類、Attributive(属性型)/Refinement(篩型)/Algebraic(代数演算型)があります。

|           | Record      | Enum       | Interval       | Union       | Intersection | Diff         |
| --------- | ----------- | ---------- | -------------- | ----------- | ------------ | ------------ |
| kind      | Attributive | Refinement | Refinement     | Algebraic   | Algebraic    | Algebraic    |
| generator | record      | set        | range operator | or operator | and operator | not operator |

記名的部分型(Nominal subtyping, NST)を使用することもでき、SST型のNST型への変換を型の記名化(Nominalization)と呼びます。こうしてできた型を記名型(Nominal type)と呼びます。
Ergでは、記名型はクラスとトレイトがそれに該当します。単にクラス/トレイトといった場合、それはレコードクラス/レコードトレイトを指す場合が多いです。

|     | Type           | Abstraction      | Subtyping procedure |
| --- | -------------- | ---------------- | ------------------- |
| NST | NominalType    | Trait            | Inheritance         |
| SST | StructuralType | Structural Trait | (Implicit)          |

記名型全体を表す型(`NominalType`)と構造型全体の型(`StructuralType`)は型全体の型(`Type`)のサブタイプです。

Ergは型定義に引数(型引数)を渡すことができます。型引数を持つ`Option`, `List`などを多項カインドと呼びます。これら自体は型ではありませんが、引数を適用することで型となります。また、引数を持たない`Int`, `Str`型などを単純型(スカラー型)と呼びます。

型は集合とみなすことができ、包含関係も存在します。例えば`Num`は`Add`や`Sub`などを含んでおり、`Int`は`Nat`を含んでいます。
全てのクラスの上位クラスは`Object == Class {:}`であり、全ての型の下位クラスは`Never == Class {}`です。これについては後述します。

## 型

`List T`のような型は型`T`を引数にとり`List T`型を返す、つまり`Type -> Type`型の関数とみなせます(型理論的にはカインドともいう)。`List T`のような型は、特に多相型(Polymorphic Type)と呼び、`List`そのものは1項カインドといいます。

引数、戻り値の型が判明している関数の型は`(T, U) -> V`のように表記します。型が同じ2引数関数全体を指定したい場合は`|T| (T, T) -> T`、N引数関数全体を指定したい場合、`Func N`で指定できる。ただし`Func N`型は引数の数や型に関する情報がないので、呼び出すと戻り値はすべて`Obj`型になります。

`Proc`型は`() => Int`などのように表記します。また、`Proc`型インスタンスの名前は最後に`!`をつけなくてはなりません。

`Method`型は第1引数に自身が属するオブジェクト`self`を(参照として)指定する 関数/プロシージャです。依存型においては、メソッド適用後の自身の型も指定できます。これは `T!(!N)`型で`T!(N ~> N-1).() => Int`などのようにメソッドを指定できるということです。

Ergの配列(List)はPythonでいうところのリストとなります。`[Int; 3]`は`Int`型オブジェクトが3つ入る配列クラスです。

> __Note__: `(Type; N)`は型であり値でもあるので、このような使い方もできます。
>
> ```python
> Types = (Int, Str, Bool)
>
> for! Types, T =>
>     print! T
> # Int Str Bool
> a: Types = (1, "aaa", True)
> ```

```python
pop|T, N|(l: [T; N]): ([T; N-1], T) =
    [*l, last] = l
    (l, last)

lpop|T, N|(l: [T; N]): (T, [T; N-1]) =
    [first, *l] = l
    (first, l)
```

`!`の付く型はオブジェクトの内部構造書き換えを許可する型です。例えば`[T; !N]`クラスは動的配列となります。
`T`型オブジェクトから`T!`型オブジェクトを生成するには、単項演算子の`!`を使います。

```python
i: Int! = !1
i.update! i -> i + 1
assert i == 2
arr = [1, 2, 3]
arr.push! 4 # ImplError:
mut_arr = [1, 2, 3].into [Int; !3]
mut_arr.push! 4
assert mut_arr == [1, 2, 3, 4]
```

## 型定義

型は以下のように定義します。

```python
Point2D = {.x = Int; .y = Int}
```

なお、`i: Int`などのように`.`を省略すると、型内で使われる非公開変数になります。しかしこれも要求属性です。
型もオブジェクトなので、型自体にも属性は存在します。このような属性を型属性といいます。クラスの場合はクラス属性ともいいます。

## 型クラス、データ型(に相当するもの)

先に述べたように、Ergにおける「型」とは大まかにはオブジェクトの集合を意味します。
以下は`+`(中置演算子)を要求する `Add`型の定義です。`R, O`はいわゆる型引数で、`Int`や`Str`など実装のある型(クラス)が入れられます。他の言語で型引数には特別な記法(ジェネリクス、テンプレートなど)が与えられていますが、Ergでは通常の引数と同じように定義できます。
なお型引数は型オブジェクト以外も使用できます。例えば配列型`[Int; 3]`は`List Int, 3`の糖衣文法です。型の実装がかぶる場合、ユーザは明示的に選択しなくてはなりません。

```python
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
```

.`_+_`は Add.`_+_`の省略形です。前置演算子の.`+_`は`Num`型のメソッドです。

```python
Num = Add and Sub and Mul and Eq
NumImpl = Patch Num
NumImpl.
    `+_`(self): Self = self
    ...
```

多相型は関数のように扱えます。`Mul Int, Str`などのように指定して単相化します(多くの場合は指定しなくても実引数で推論されます)。

```python
1 + 1
`_+_` 1, 1
Nat.`_+_` 1, 1
Int.`_+_` 1, 1
```

上の4行は同じ結果を返しますが(正確には、一番下は`Int`を返します)、一番上を使うのが一般的です。
```Ratio.`_+_`(1, 1)```とすると、エラーにはならず`2.0`が返ります。
これは、`Int <: Ratio`であるために`1`が`Ratio`にダウンキャストされるからです。
しかしこれはキャストされません。

```python
i = 1
if i: # TypeError: i: Int cannot cast to Bool, use Int.is_zero() instead.
    log "a"
    log "b"
```

これは、`Bool < Int`であるためです(`True == 1`, `False == 0`)。サブタイプへのキャストは一般に検証が必要です。

## 型推論システム

Ergは静的ダックタイピングを採用しており、明示的に型を指定する必要は殆どありません。

```python
f x, y = x + y
```

上のコードの場合、`+`を持つ型、すなわち`Add`が自動的に推論されます。Ergはまず最小の型を推論します。`f 0, 1`とすれば`f x: {0}, y: {1}`と推論され、`n: Nat; f n, 1`の場合`f x: Nat, y: {1}`と推論されます。最小化後は実装が見つかるまで型を大きくしていきます。`{0}, {1}`の場合`Nat`が`+`の実装がある最小型なので`Nat`に単相化されます。
`{0}, {-1}`の場合は`Nat`にマッチしないので`Int`に単相化されます。部分型、上位型の関係にない場合は、濃度(インスタンス数)が低い(多相型の場合はさらに引数の少ない)方からトライされます。
`{0}`と`{1}`は`Int`や`Nat`などの部分型となる列挙型です。
列挙型などには名前を付けて要求/実装メソッドを付けられます。その型にアクセスできる名前空間では、要求を満たすオブジェクトは実装メソッドを使用できます。

```python
Binary = Patch {0, 1}
Binary.
    # selfにはインスタンスが格納される。この例では0か1のどちらか。
    # selfを書き換えたい場合、型名、メソッド名に`!`を付けなければならない。
    is_zero(self) = match self:
        0 -> True
        1 -> False # _ -> Falseとしてもよい
    is_one(self) = not self.is_zero()
    to_bool(self) = match self:
        0 -> False
        1 -> True
```

以降は`0.to_bool()`というコードが可能となります(もっとも`0 as Bool == False`がビルトインで定義されていますが)。
コード中に示されたように、実際に`self`を書き換える事のできる型の例を示します。

```python
Binary! = Patch {0, 1}!
Binary!.
    switch! ref! self = match! self:
        0 => self = 1
        1 => self = 0

b = !1
b.switch!()
print! b # => 0
```

## 構造型(無名型)

```python
Binary = {0, 1}
```

上のコードでの`Binary`は、`0`および`1`が要素の型です。`0`と`1`両方を持っている`Int`型の部分型とも言えます。
`{}`のようなオブジェクトはそれ自体が型であり、上のように変数に代入して使ってもよいし、代入せずに使うこともできます。
このような型を構造型といいます。クラス(記名型)と対比して後者としての使い方を強調したいときは無名型ともいいます。`{0, 1}`のような種類の構造型は列挙型と呼ばれ、他に区間型、レコード型などがあります。

### 型同一性

下のような指定はできません。`Add`はそれぞれ別のものを指すと解釈されるからです。
例えば、`Int`と`Str`はともに`Add`だが、`Int`と`Str`の加算はできません。

```python
add l: Add, r: Add =
    l + r # TypeError: there is no implementation of  `_+_`: |T, U <: Add| (T, U) -> <Failure>
```

また、下の`A`, `B`は同じ型とはみなされません。しかし、型`O`は一致するとみなされます。

```python
... |R1; R2, O; A <: Add(R1, O); B <: Add(R2, O)|
```

<p align='center'>
    Previous | <a href='./02_basic.md'>Next</a>
</p>
