# 可変性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/17_mutability.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/17_mutability.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

すでに見たように、Ergの変数は全て不変です。しかし、Ergのオブジェクトには可変性という概念があります。
以下のコードを例にします。

```python
a = [1, 2, 3]
a = a + [4, 5, 6]
print! a # [1, 2, 3, 4, 5, 6]
```

上のコードは実際にはErgでは実現できません。再代入不可だからです。
このコードは実行できます。

```python
b = ![1, 2, 3]
b.concat! [4, 5, 6]
print! b # [1, 2, 3, 4, 5, 6]
```

`a, b`は、最終的な結果は同じように見えますが、その意味は大きく異なります。
`a`は`Nat`の配列を示す変数ですが、1行目と2行目では指しているオブジェクトが異なります。`a`という名前が同じだけで、中身はさし変わっているのです。

```python
a = [1, 2, 3]
print! id! a # 0x000002A798DFE940
_a = a + [4, 5, 6]
print! id! _a # 0x000002A798DFE980
```

`id!`プロシージャはオブジェクトが存在するメモリ上のアドレスを返します。

`b`は`Nat`の「動的」配列です。オブジェクトの中身は変わりますが、変数の指すものは同じです。

```python
b = [1,2,3].into [Int; !3]
print! id! b # 0x000002A798DFE220
b.concat! [4, 5, 6]
print! id! b # 0x000002A798DFE220
```

```python
i = !0
if! True:
    do! i.inc!() # or i.add!(1)
    do pass
print! i # 1
```

`!`は __可変化演算子(mutation operator)__ とよばれる特殊な演算子です。引数の不変オブジェクトを可変化して返します。
`!`がついたオブジェクトの振る舞いはカスタム可能です。

```python
Point = Class {.x = Int; .y = Int}

# この場合.xは可変化し、yは不変のまま
Point! = Class {.x = Int!; .y = Int}
Point!.inc_x! ref! self = self.x.update! x -> x+1

p = Point!.new {.x = !0; .y = 0}
p.inc_x!()
print! p.x # 1
```

## 定数

変数と違い、すべてのスコープで同じものを指すのが定数です。
定数は`=`演算子で宣言します。

```python
PI = 3.141592653589
match! x:
    PI => print! "this is pi"
```

定数はグローバル以下のすべてのスコープで同一であり、上書きができません。よって、`=`による再定義はできません。この制限により、パターンマッチで使うことができます。
`True`や`False`がパターンマッチで使えるのは、この２つが定数だからなのです。
また、定数は必ず不変オブジェクトを指しています。`Str!`型などは定数となれません。
組み込み型がすべて定数なのは、コンパイル時に決定されているべきだからです。定数でない型も生成可能ですが、型指定には使えず、単なるレコードのようにしか使えません。逆に言えば、型はコンパイル時に内容が決定されているレコードとも言えるでしょう。

## 変数、名前、識別子、シンボル

ここで、Ergでの変数に関する用語を整理しておきましょう。

変数(Variable)はオブジェクトに名前(Name)をつけ、再利用できるようにする仕組み(またはその名前を指す)です。
識別子(Identifier)は変数を指定する文法要素です。
シンボルは名前を表すための文法要素、トークンです。

記号でない文字だけがシンボルであり、記号は演算子として識別子足り得ますが、シンボルとは呼びません。
例えば、`x`は識別子でシンボルです。`x.y`も識別子ですが、これはシンボルとは言いません。`x`と`y`はシンボルです。
また`x`が何のオブジェクトに紐づけられていなかったとしても、`x`は相変わらずSymbolかつIdentifierですが、Variableとは言いません。
`x.y`という形の識別子はフィールドアクセサと言います。
また、`x[y]`という形の識別子は添字アクセサと言います。

変数と識別子の違いですが、Ergの文法論的な意味での変数をいうのならば、実質この二つは同じです。
変数と識別子が等価でない言語は、C言語などがあげられます。C言語では、型や関数は変数に代入できません。int, mainは識別子ですが変数ではないのです(厳密には代入出来る場合もありますが、制約があります)。
しかし、Ergでは「全てがオブジェクト」です。関数や型は勿論、演算子でさえ変数に代入可能です。

<p align='center'>
    <a href='./16_iterator.md'>Previous</a> | <a href='./18_ownership.md'>Next</a>
</p>
