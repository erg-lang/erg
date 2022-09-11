# 用語辞典

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/terms.md%26commit_hash%3D275c35f66b250942fda1ab0cee173ea016e9fd67)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/terms.md&commit_hash=275c35f66b250942fda1ab0cee173ea016e9fd67)

## 記号

### &excl;

プロシージャ、または可変型であることを示すために識別子の末尾に付与するマーカー。
または、可変化演算子。

### [&#35;](../syntax/00_basic.md/#コメント)

### $

### %

### &

### &prime; (single quote)

### &lpar;&rpar;

### &ast;

### &plus;

### &comma;

### &minus;

### ->

### &period;

### /

### &colon;

### &colon;&colon;

### &semi;

### &lt;

### &lt;&colon;

### &lt;&lt;

### &lt;=

### =

### ==

### =>

### &gt;

### &gt;&gt;

### &gt;=

### ?

### @

### []

### \

### ^

### ^^

### _

### ``

### {}

### {:}

### {=}

### |

### ||

### ~

## A

### [algebraic&nbsp;type]

### [And]

### [and]

### [assert]

### [attribute]

## B

### [Base]

### [Bool]

## C

### [Class]

## D

### Deprecated

### [distinct]

## E

### [enum&nbsp;type]

### [Eq]

### [Erg]

## F

### [for]

## G

## H

## I

### [if]

### [import]

### [in]

### [Int]

## J

## K

## L

### let-polymorphism -> [ランク1多相]

### [log]

## M

### [match]

## N

### [Nat]

### Never

### None

### [Not]

### [not]

## O

### [Option]

### [Or]

### [or]

### [Ord]

## P

### panic

### [print!](../syntax/../API/procs.md#print)

### [Python]

## Q

## R

### ref

### ref&excl;

### [Result]

### [rootobj]

## S

### self

### [Self](../syntax/type/special.md)

### [side-effect](../syntax/07_side_effect.md)

### [Str]

## T

### Trait

### [True]

### [Type]

### [type]

## U

## V

## W

### [while!]

## X

## Y

## Z

## あ行

### [アサーション]

コード中である条件が成立しているか(典型的には実行時に)調べること。`assert`関数などを使用して行う。

```python
sum = !0
for! 0..10, i =>
    sum.add! i

assert sum == 55
```

### 値オブジェクト

Ergにおいては、基本オブジェクトと同等。コンパイル時に評価でき、自明な比較方法を持つ。

### [アタッチメントパッチ](../syntax/29_decorator.md#attach)

トレイトに標準の実装を与えるパッチ。

### アドホック多相 -> [オーバーロードの禁止](../syntax/type/overloading.md)

いわゆるオーバーロードによる多相。

### アトリビュート -> [属性]

`x.y`という識別子における`y`の部分。

### アリティ

演算子がいくつのオペランドを取るか。

### [依存型](../syntax/type/dependent_type.md)

値(慣用的には、型ではない)を引数にとる型。

### イミュータブル -> [不変]

対象が変更されないことを示す。
他の言語では変数にもイミュータブル/ミュータブル性があるが、Ergでは変数はすべてイミュータブル。

### 引数(いんすう) -> [引数(ひきすう)]

### インスタンス

クラスによって作られたオブジェクト。クラス型の要素。

### [インスタントブロック](../syntax/00_basic.md#式セパレータ)

```python
x =
    y = f(a)
    z = g(b, c)
    y + z
```

### インデックス

`x[i]`という形式、またはそれにおける`i`の部分。`x`をIndexableオブジェクトという。

### [インデント](../syntax/00_basic.md#インデント)

スペースに寄って文を右に寄せること。字下げ。
Ergはインデントによってブロックを表現する。これをオフサイドルールという。

### エイリアス

別名のこと。

### エラー

仕様で定められた異常状態。

* [エラーハンドリング]

### [演算子](../syntax/06_operator.md)

オペランドに演算を適用するオブジェクト。またはそのオブジェクトを示す記号。

* [演算子の結合強度]

### オーバーライド

サブクラスでスーパークラスのメソッドを上書きすること。
Ergではオーバーライドの際`Override`デコレータを付けなくてはならない。

### [オーバーロードの禁止](../syntax/type/overloading.md)

### オフサイドルール -> [インデント](../syntax/00_basic.md#インデント)

### [オブジェクト]

* オブジェクト指向

### オペランド -> [被演算子](../syntax/06_operator.md)

### オペレーター -> [演算子](../syntax/06_operator.md)

## か行

### [カインド](../syntax/type/advanced/kind.md)

いわゆる型の型。

### [可視性]

識別子が外部(スコープ外、または別モジュール、別パッケージ)から参照可能であるかという性質。

### [型]

項をグルーピングするオブジェクト。

* [型指定]
* [型消去](../syntax/type/advanced/erasure.md)
* [型推論]
* [型注釈](../syntax/type/conv_type.md)
* [型引数]
* [型付加](../syntax/type/advanced/erasure.md)
* [型変数](../syntax/type/type_variable.md)
* [型制約]

### [ガード]

### カプセル化

実装詳細を隠蔽すること。

### [可変]

イミュータブルでないこと。

* [可変オブジェクト]
* [可変型]
* [可変参照]
* [可変配列]
* [可変長引数]

### [関数](../syntax/04_function.md)

副作用のないサブルーチン。

* [関数型プログラミング](../syntax/23_scope.md#可変状態の回避関数型プログラミング)

### 基底型

### 記名的

対称の構造ではなく名前によって区別すること。

* [記名型] -> [クラス](../syntax/type/04_class.md)
* [記名化]
* [記名的部分型](../syntax/type/05_nst_vs_sst.md)

### キャプチャ -> [クロージャ]

### [共変]

Ergにおいては、`T <: U`のとき`K(T) <: K(U)`ならば`K`は共変であるという。

### [キーワード引数]

関数呼び出し`f(k: v)`の形式における`k`のこと。実引数を順番ではなく仮引数名で指定できる。

### 空集合 -> [{}]

### 区間

* [区間型](../syntax/type/11_interval.md)
* 区間演算子

### 組み込み

Erg標準APIのうち、.erファイル内で実装されていないAPI。

### [クラス](../syntax/type/04_class.md)

継承機能を持つ構造体・抽象データ型。Ergにおいては記名的サブタイピング、およびオーバーライドを実現するための型である。
他の言語ではモジュールや型の責務を担う場合もあるが、Ergにおいては、モジュールはモジュールオブジェクト、型は型オブジェクトがその責務を担う。

### [クロージャ]

### [グローバル変数]

### [クローン]

### [継承](../syntax/type/07_inheritance.md)

あるクラスを上位集合としたクラスを定義すること。
継承元のクラスはスーパークラス、継承先のクラスはサブクラスと呼ばれる。
サブクラスはスーパークラスの機能をすべて持つ。

### 高階

* [高階カインド](../syntax/type/advanced/kind.md)
* 高階型
* 高階関数

### [公開変数]

### [構造的部分型]

### ~~後方参照~~ -> [前方参照]

### [コピー]

### コメント

### [コレクション](../syntax/10_array.md)

### コロン -> [:]

### [コンストラクタ](../syntax/type/04_class.md)

### コンテナ

### コンパイラ

### [コンパイル時計算](../syntax/04_function.md#コンパイル時関数)

### コンマ -> [,]

## さ行

### 再帰

自身を参照すること。

* 再帰型
* [再帰関数](../syntax/04_function.md#再帰関数)

### サブスクリプト -> [インデックス]

### [サブタイピング多相](../syntax/type/overloading.md)

サブタイピングによる多相。サブタイピングとは、型において集合の包含関係に対応するものである。

### サブルーチン

処理をモジュール化したオブジェクト。Ergでは関数、プロシージャ、およびメソッドの総称。

### [参照](../syntax/18_memory_management.md#借用)

* 参照オブジェクト
* [参照カウント(RC)](../syntax/18_memory_management.md#メモリ管理)
* 参照等価性 -> [副作用](../syntax/07_side_effect.md)

### [識別子](../syntax/02_variable.md/#代入)

### シグネチャ

* 型シグネチャ

### [辞書](../syntax/11_dict.md)

### [自然数] -> [Nat]

### ジェネリクス -> [全称型]

### ジェネレータ

### [射影型]

### 借用 -> [参照](../syntax/18_memory_management.md#借用)

### [シャドーイング](../syntax/02_name.md#変数)

ある変数に対し、内側のスコープで同名の変数を定義してその参照を上書きすること。

### 種 -> [カインド](../syntax/type/advanced/kind.md)

おおまかには型の型。

### [集合] -> [セット]

ErgにおいてはSetオブジェクトのこと。

### 述語

* [述語関数]

Bool型を返す関数。

### 条件分岐

### [所有権]

オブジェクトのユニーク性に関する概念。
オブジェクトの所有権を持つ場合、オブジェクトの可変参照を取ることができる。

### 真偽型 -> [Bool]

### シングルトン

インスタンスを一つしか生成できないクラスから生成されたインスタンス。また、クラスのインスタンスが1つしか生成されないことを保証するデザインパターンのこと。

### [シンボル] -> [識別子](../syntax/02_name.md)

* [シンボル化]

### [スクリプト](../syntax/00_basic.md#スクリプト)

Ergプログラムが記述されたファイル。

### スコープ

変数管理における単位。外側のスコープでは内側のスコープに存在する変数を参照できない。
また、スコープを抜けたときに、参照カウントが0であるオブジェクトは解放される。

### スプレッド演算子 -> [展開代入]

### [スライス](../syntax/10_array.md#スライス)

`x[a..b]`の形式で生成される、配列の部分列を表すオブジェクト。

### 制御文字

### [整数] -> [Int]

自然数に負数を合わせた集合。

### [セット](../syntax/12_set.md)

### セミコロン -> [;]

### [宣言](../syntax/03_declaration.md)

変数を明示的に型付けること。

### 全称

* 全称型 -> [多相型](../syntax/type/quantified.md)
  * 閉じた全称型
  * 開いた全称型
* 全称関数 -> 多相関数
* 全称量化

### 前置演算子

`∘x`の形式で適用される演算子`∘`。

### 相互再帰

### 添字 -> [インデックス]

### [属性]

* [属性的部分型]

## た行

### [代数](../syntax/02_name.md)

* [代数演算型](../syntax/type/13_algebraic.md)
* 代数的データ型

### [代入](../syntax/02_variable.md/#代入)

### 多重

* [多重継承](../syntax/type/07_inheritance.md/#多重継承の禁止)
* 多重代入
* 多重定義 -> [オーバーロードの禁止]

### 多相

* [多相型](../syntax/type/quantified.md)
* 多相関数

### 多態 -> [ポリモーフィズム]

### ダックタイピング

### [タプル](../syntax/11_tuple.md)

### 単相

* 単相化
* 単相型
* 単相関数

### [遅延初期化]

### 抽出代入

### 抽象構文木 -> [AST]

### 中置演算子

`x∘y`の形式で適用される演算子`∘`。

### [定数](../syntax/02_name.md/#定数)

イミュータブルでコンパイル時評価可能な代数。

* [定数型](../syntax/type/advanced/const.md)
* [定数式](../syntax/type/advanced/const.md)

### [定義]

変数に対応するオブジェクトを割り当てること。

### 提供属性

APIとして利用可能な属性。特に、トレイトによって自動実装された属性。

### [適用]

関数オブジェクトに引数を渡して評価結果を得ること。

### [デコレータ](../syntax/29_decorator.md)

```python
@deco
f x = ...
```

という糖衣構文、または`deco`のこと。`_f x = ...; f = deco _f`とおよそ等しい。`deco`自体は単なる高階サブルーチンにすぎない。

### デストラクタ

オブジェクトが破棄されるときに呼ばれるメソッド。

### 手続き -> [プロシージャ](../syntax/08_procedure.md)

サブルーチンのうち、可変状態を読み書きするもの。
プロシージャは呼出順序によってプログラムの実行結果が変わりうるという説明がなされることがあるが、これは可換性のことを言っているならば誤りである。
例えば関数のサブタイプである演算子は一般に可換でない。

### [デフォルト引数](../syntax/04_function.md/#デフォルト引数default-parameters)

仮引数にデフォルトの値を指定することで、呼び出しの際に実引数の指定を省略できる機能。

### 展開

* [展開演算子]
* [展開代入]

### [特殊形式](../syntax/../API/special.md)

実引数に渡すことができないオブジェクト。

### 匿名関数 -> [無名関数](../syntax/20_lambda.md)

無名関数演算子`->`によって生成される関数オブジェクト。名前を定義せずに使える。

### ドット演算子(`.`) -> [属性参照]

### トップ

* トップ型 -> [Structural Object]
* トップクラス -> [Object]

### [トレイト](../syntax/type/03_trait.md)

## な行

### [内包表記](../syntax/27_comprehension.md)

### ~~中置(なかおき)演算子~~ -> [中置(ちゅうち)演算子]

### [名前空間]

## は行

### [配列](../syntax/10_array.md)

### [派生型](../syntax/type/variances.md/#ユーザー定義型の変性)

### [パターン(マッチ)](../syntax/26_pattern_matching.md)

### [パッケージ](../syntax/33_package_system.md)

### ハッシュマップ -> [辞書](../syntax/11_dict.md)

### [パッチ](../syntax/type/07_patch.md)

### パブリック変数 -> [公開変数](../syntax/19_visibility.md)

### パラメーター -> [引数](../syntax/04_function.md)

### [パラメトリック多相](../syntax/type/overloading.md)

### [反変](../syntax/type/advanced/variance.md)

### 比較

* [比較演算子]
* [比較可能型]

### [非公開変数](../syntax/19_visibility.md)

### 標準

* 標準出力
* 標準入力
* 標準ライブラリ

### [副作用](../syntax/07_side_effect.md)

コードが外部の可変状態を読み書きする/しないこと。

### 複素数 -> [Complex]

### [浮動小数点数] -> [Float]

### プライベート変数 -> [非公開変数]

### ブール代数 -> [Bool]

### [プロシージャ](../syntax/08_procedure.md)

### [引数](../syntax/04_function.md)

### 部分型付け -> [サブタイピング]

### [不変]

Ergにおいては、オブジェクトがその内容を変えないこと。

* [不変オブジェクト]
* [不変型]
* [不変参照]

### [篩型](../syntax/type/12_refinement.md)

### [ブロック]

### 分解代入

### [変数](../syntax/02_variable.md)

### ボトム

* ボトム型 -> [{}]
* ボトムクラス -> [Never]

### [ポリモーフィズム]

## ま行

### ~~前置(まえおき)演算子~~ -> 前置(ぜんち)演算子

### [マーカー型](../syntax/type/advanced/marker_trait.md)

### [無名関数](../syntax/21_lambda.md)

### ミュータブル -> [可変性]

### [ムーブ]

### メソッド

### メタキャラクタ

### [モジュール](../syntax/24_module.md)

### [文字列] -> [Str]

* [文字列補間](../syntax/01_literal.md/#Strリテラル)

### 戻り値

## や行

### [幽霊型](../syntax/type/advanced/phantom.md)

### 要求属性

### [要素]

### [呼び出し]

## ら行

### [ライブラリ]

### ラムダ式 -> [無名関数](../syntax/20_lambda.md)

### ランク

* [ランク2多相](../syntax/type/advanced/rank2type.md)

### [リテラル](../syntax/01_literal.md)

* [リテラル識別子](../syntax/18_naming_rule.md/#リテラル識別子)

### [量化](../syntax/type/quantified.md)

### [レイアウト](../syntax/type/mut.md)

### [列挙型](../syntax/type/10_enum.md)

### [レコード](../syntax/12_record.md)

* [レコード型]
* レコード多相 -> [列多相]

### [列多相]

### [ローカル変数](../syntax/19_visibility.md)

## わ行

### ワイルドカード
