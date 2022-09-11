# 用語の統一

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/unify_terms.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/unify_terms.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## Accessibility, Visibility (参照性、可視性)

Visibility(可視性)を使用する。

## Complement(否定型、補型)

否定型を使用する。Complementの結果Not型になるとは限らない。

## Diff(差型、除外型、直差型)

除外型を使用する。Diffの結果Not型になるとは限らない。

## Intersection(共通部分型、交差型、直積型)

交差型を使用する。直積型は使用しない。タプルを直積型とみなす用法もあるためである。
ただし、属性的部分型の観点からはErgのAnd型と本質的に等価な概念である。
また、Intersectionの結果And型になるとは限らない。例えば`{1, 2, 3} and {1, 2} == {1, 2}`である。

## Nominal subtypingの訳語

記名的/名目的/公称的部分型付けがあるが、記名的部分型付けを使用する。

## Ratio型の訳語

有理数型を使用する。Floatは別途提供されているので、浮動小数点数型とは呼ばない。

## Union(合併型、直和型)

合併型を使用する。Unionの結果Or型になるとは限らない。

## 型境界(Type bound)、型制約(Type constraint)

量化型、篩型に与えられている述語式のリスト。型境界を使用する。

## サブルーチン、ルーチン、サブプログラム

サブルーチンを使用する。

## 参照透過である/でない、副作用あり/なし

副作用あり/なしを使用する。

## 識別子、代数、変数、名前、シンボル

元来の意味としては、

* シンボル(Symbol): 文字列オブジェクトでない(""で囲まれていない)ソースコードにベタ書きされた文字(記号や制御文字などを除く)。RubyやLispなどでのプリミティブ型としてのシンボルが存在するが、Ergではオブジェクト扱いされない。
* 識別子(Identifier): シンボルのうち、予約語でなく、何らかのオブジェクトを指すもの(また指し得るもの)。例えばPythonではclassやdefは識別子として使えない。Ergには予約語がないため、一部の記号を除くすべてのシンボルが識別子として使える。
* 名前(Name): 識別子とほぼ同じ意味。Ergにおいては代数と同じ意味で使われることもある。
* 代数名(Algebra name): Ergにおいては識別子と同等の意味。C言語では関数名は識別子だが代数名ではない。「代数」は`=`(変数代入演算子)または`=`(定数代入演算子)でオブジェクトを代入できるという言語機能自体を指す。

```python
代数名 <: (名前 == 識別子) <: シンボル
変数 + 定数 == 代数
```

ただし、本来「代数」と呼ばれるべきものは「変数」と呼ばれる場合が多い。これは数学用語の影響である。
値の内容が変わりうる変数はミュータブル変数、値の内容が変わらない変数はイミュータブル変数である。
なお、定数は必ずイミュータブルである。

Ergでは代数名、名前は使用せず、識別子で統一する。
ただし一般的には`v = 1`の`v`は「変数v」("Variable v")と呼び、`C = 1`の`C`は「定数C」("Constant C")と呼ぶ。

## 属性、フィールド、プロパティ(Attribute, Field, Property)

Attribute、属性を使用する。
因みにレコードはクラス無しで要素属性のあるオブジェクトを定義できる機能のことである。

## 適用(Application)、呼び出し(Call)

サブルーチンオブジェクトに引数を与えて結果を得ること。
呼び出し(Call)を使用する。Applicationは「応用ソフトウェア」の用法があるためである。

## 配列(Array)、リスト(List)

Arrayを使用する。Ergの配列は(一般的には)メモリ上で連続的に配置されるからである。
Listはいわゆる連結リスト、またはPythonのデータ型としてのリストを指すこととする。

## プロシージャ、手続き

プロシージャに統一する。サブルーチンは関数(と演算子)、プロシージャ、メソッドの総称。Callableはさらに`__call__`を実装しているものすべて。

## ラムダ関数、ラムダ式、匿名関数、無名関数

無名関数で統一する。英語では字数短縮のためLambdaを使用してよいが、正式名はAnonymous functionである。
また、Ergの無名関数は匿名なわけではないので、匿名関数は使わない。
