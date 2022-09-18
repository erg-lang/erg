# 変数

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/02_name.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/02_name.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

変数は代数の一種です。Ergにおける代数―紛れがなければ単に変数と呼ばれることもあります―とは、オブジェクトを名前付けしてコード中の別の場所から利用できるようにする機能を指します。

変数は以下のように定義します。
`n`の部分を変数名(または、識別子)、`=`を代入演算子、`1`の部分を代入値と呼びます。

```python
n = 1
```

このようにして定義した`n`は、以降整数オブジェクトである`1`を示す変数として使用できます。このシステムを代入(または束縛)といいます。
いま`1`はオブジェクトであると述べました。オブジェクトが何であるかは後述しますが、今は代入できるもの、すなわち代入演算子(`=`など)の右側におけるものとしておきます。

変数の「型」を指定したい場合は以下のようにします。型とは、これも後述しますが、大まかにはオブジェクトの属する集合です。
ここでは`n`は自然数(`Nat`)型であると指定しています。

```python
n: Nat = 1
```

他の言語とは違い、多重代入はできないので注意してください。

```python
# NG
l1 = l2 = [1, 2, 3] # SyntaxError: multiple assignment not allowed
# OK
l1 = [1, 2, 3]
l2 = l1.clone()
```

また、変数への再代入もできません。その代わりに使える機能、すなわち可変な状態を保持する機能については後述します。

```python
i = 1
i = i + 1 # AssignError: cannot assign twice
```

内側のスコープで同じ名前の変数を定義できますが、上に被せているだけで、値を破壊的に書き換えているわけではありません。外側のスコープに戻れば値も戻ります。
これはPythonの「文」のスコープとは違う挙動なので注意してください。
このような機能は一般にシャドーイングと言います。ただし他言語のシャドーイングとは違い同一スコープではシャドーイングできません。

```python
x = 0
# x = 1 # AssignError: cannot assign twice
if x.is_zero(), do:
    x = 1 # 外側のxとは同名の別物になる
    assert x == 1
assert x == 0
```

以下は一見すると可能なように思えますが、やはりできません。これは技術的な制約ではなく、設計判断です。

```python
x = 0
if x.is_zero(), do:
    x = x + 1 # NameError: cannot define variables refer to variables with the same name
    assert x == 1
assert x == 0
```

## 定数

定数も代数の一種です。識別子を大文字で始めると定数として扱われます。一度定義したら変わらないので、定数と呼ばれます。
`N`の部分を定数名(または、識別子)と呼びます。その他は変数と同じです。

```python
N = 0
if True, do:
    N = 1 # AssignError: constants cannot be shadowed
    pass()
```

定数は定義されたスコープ以降では不変になります。シャドーイングもできません。この性質から、定数はパターンマッチで使用できます。
パターンマッチについては後に説明します。

定数は、数学的定数、外部リソースに関する情報など不変な値に対して使用すると良いでしょう。
[型](./type/01_type_system.md)以外のオブジェクトは、オールキャップス(全ての文字を大文字にするスタイル)にするのが一般的です。

```python
PI = 3.141592653589793
URL = "https://example.com"
CHOICES = ["a", "b", "c"]
```

```python
PI = 3.141592653589793
match! x:
    PI => print! "π"
    other => print! "other"
```

上のコードは`x`が`3.141592653589793`のとき`π`を出力します。`x`を他の数字に変えると、`other`を出力します。

定数には代入できないものがあります。可変オブジェクトなどです。詳しくは後述しますが、可変オブジェクトは内容を変更することができるオブジェクトです。
これは定数には定数式のみを代入できるという規則があるためです。定数式についても後述することとします。

```python
X = 1 # OK
X = !1 # TypeError: cannot define Int! object as a constant
```

## 代数の削除

`Del`関数を使うことで、代数を削除することが出来ます。その代数に依存している(その代数の値を直接参照している)他の代数もまとめて削除されます。

```python
x = 1
y = 2
Z = 3
f a = x + a

assert f(2) == 3
Del x # xを直接参照しているためfも削除される
Del y, Z

f(2) # NameError: f is not defined (deleted in line 6)
```

ただし、`Del`によって削除できるのはモジュール内で定義された代数のみです。`True`などの組み込み定数は削除できません。

```python
Del True # TypeError: cannot delete built-in constants
Del print! # TypeError: cannot delete built-in variables
```

## 付録: 代入と同値性

注意として、`x = a`であるとき、`x == a`とは限らない。例としては`Float.NaN`がある。これはIEEE 754により定められた正式な浮動小数点数の仕様である。

```python
x = Float.NaN
assert x != Float.NaN
assert x != x
```

その他、そもそも同値関係が定義されていないオブジェクトも存在する。

```python
f = x -> x**2 + 2x + 1
g = x -> (x + 1)**2
f == g # TypeError: cannot compare function objects

C = Class {i: Int}
D = Class {i: Int}
C == D # TypeError: cannot compare class objects
```

厳密に言うと`=`は右辺値をそのまま左辺の識別子に代入するわけではない。
関数オブジェクトやクラスオブジェクトの場合、オブジェクトに変数名の情報を与えるなどの「修飾」を行う。
ただし構造型の場合はその限りではない。

```python
f x = x
print! f # <function f>
g x = x + 1
print! g # <function g>

C = Class {i: Int}
print! C # <class C>
```

<p align='center'>
    <a href='./01_literal.md'>Previous</a> | <a href='./03_declaration.md'>Next</a>
</p>
