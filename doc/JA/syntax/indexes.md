# 索引

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/indexes.md%26commit_hash%3D68054846e20b4cdb0e92e986b1b86fcc77de8bcd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/indexes.md&commit_hash=68054846e20b4cdb0e92e986b1b86fcc77de8bcd)

この索引にないAPIについては[こちら](../API/index.md)を参照してください。
用語の意味については[こちら](../dev_guide/terms.md)を参照。

## 記号

* !
  * !-type → [可変型](./type/mut.md)
* [&#35;](./00_basic.md/#コメント)
* $
* %
* &
  * &&
* &prime; (single quote)
* &lpar;&rpar;
* &ast;
  * [*-less multiplication](./01_literal.md/#less-multiplication)
* &plus; (前置)
  * &plus;_ → &plus; (前置)
* &plus; (中置)
* ,
* &minus; (前置)
  * &minus;_ → &minus; (前置)
* &minus; (中置)
  * &minus;>
* . → [可視性]
* /
* :
  * :: → [可視性]
* ;
* &lt;
  * &lt;:
  * &lt;&lt;
  * &lt;=
* =
  * ==
  * =>
* &gt;
  * &gt;&gt;
  * &gt;=
* ?
* @
* []
* \
* ^
  * ^^
* _
  * &#95;+&#95; → &plus; (中置)
  * &#95;-&#95; → &minus; (中置)
* ``
* {}
  * {} type
* {:}
* {=}
  * {=} type
* |
  * ||
* ~

## アルファベット

### A

* [algebraic&nbsp;type]
* [And]
* [and]
* [assert]
* [attribute]

### B

* [Base]
* [Bool]

### C

* [Class]

### D

* Deprecated
* [distinct]

### E

* [enum&nbsp;type]
* [Eq]
* [Erg]

### F

* [for]

### G

### H

### I

* [if]
* [import]
* [in]
* [Int]

### J

### K

### L

* let-polymorphism → [ランク1多相]
* [log]

### M

* [match]

### N

* [Nat]
* Never
* None
* None
* [Not]
* [not]

### O

* [Option]
* [Or]
* [or]
* [Ord]

### P

* panic
* [print!](./../API/procs.md#print)
* [Python]

### Q

### R

* ref
* ref!
* [Result]
* [rootobj]

### S

* self
* [Self](./type/special.md)
* [side-effect](./07_side_effect.md)
* [Str]

### T

* Trait
* [True]
* [Type]
* [type]

### U

### V

### W

* [while!]

### X

### Y

### Z

## あ行

* [アサーション]
* 値オブジェクト
* [アタッチメントパッチ](./29_decorator.md#attach)
* アドホック多相 → [オーバーロードの禁止](./type/overloading.md)
* アトリビュート → [属性]
* アリティ
* [依存型](./type/dependent_type.md)
* イミュータブル → [不変]
* 引数(いんすう) → [引数(ひきすう)]
* インスタンス
* [インスタントブロック](./00_basic.md#式セパレータ)
* インデックス
* [インデント](./00_basic.md#インデント)
* エイリアス
* エラー
  * [エラーハンドリング]
* [演算子](./06_operator.md)
  * [演算子の結合強度]
* オーバーライド
* [オーバーロードの禁止](./type/overloading.md)
* オフサイドルール → [インデント](./00_basic.md#インデント)
* [オブジェクト]
  * オブジェクト指向
* オペランド → [被演算子](./06_operator.md)
* オペレーター → [演算子](./06_operator.md)

## か行

* [カインド](./type/advanced/kind.md)
* [可視性]
* [型]
  * [型指定]
  * [型消去](./type/advanced/erasure.md)
  * [型推論]
  * [型注釈](./type/conv_type.md)
  * [型引数]
  * [型付加](./type/advanced/erasure.md)
  * [型変数](./type/type_variable.md)
  * [型制約]
* [ガード]
* カプセル化
* [可変]
  * [可変オブジェクト]
  * [可変型]
  * [可変参照]
  * [可変配列]
  * [可変長引数]
* [関数](./04_function.md)
  * [関数型プログラミング](./23_scope.md#可変状態の回避関数型プログラミング)
* 基底型
* 記名
  * [記名型] → [クラス](./type/04_class.md)
  * [記名化]
  * [記名的部分型](./type/05_nst_vs_sst.md)
* キャプチャ → [クロージャ]
* [共変]
* [キーワード引数]
* 空集合 → [{}]
* 区間
  * [区間型](./type/11_interval.md)
  * 区間演算子
* 組み込み
  * [組み込み型]
  * [組み込み関数](./05_builtin_funcs.md)
  * [組み込みプロシージャ](./09_builtin_procs.md)
* [クラス](./type/04_class.md)
* [クロージャ]
* [グローバル変数]
* [クローン]
* [継承](./type/07_inheritance.md)
* 高階
  * [高階カインド](./type/advanced/kind.md)
  * 高階型
  * 高階関数
* [公開変数]
* [構造的部分型]
* ~~後方参照~~ → [前方参照]
* [コピー]
* コメント
* [コレクション](./10_array.md)
* コロン → [:]
* [コンストラクタ](./type/04_class.md)
* コンテナ
* コンパイラ
* [コンパイル時計算](./04_function.md#コンパイル時関数)
* コンマ → [,]

## さ行

* 再帰
  * 再帰型
  * [再帰関数](./04_function.md#再帰関数)
* サブスクリプト → [インデックス]
* [サブタイピング多相](./type/overloading.md)
* サブルーチン
* [参照](./18_memory_management.md#借用)
  * 参照オブジェクト
  * [参照カウント(RC)](./18_memory_management.md#メモリ管理)
  * 参照等価性 → [副作用](./07_side_effect.md)
* [識別子](./02_variable.md/#代入)
* シグネチャ
  * 型シグネチャ
* [辞書](./11_dict.md)
* [自然数] → [Nat]
* ジェネリクス → [全称型]
* ジェネレータ
* [射影型]
* 借用 → [参照](./18_memory_management.md#借用)
* [シャドーイング](./02_name.md#変数)
* 種 → [カインド](./type/advanced/kind.md)
* [集合] → [セット]
* 述語
  * [述語関数]
* 条件分岐
* [所有権]
* 真偽型 → [Bool]
* シングルトン
* [シンボル] → [識別子](./02_name.md)
  * [シンボル化]
* [スクリプト](./00_basic.md#スクリプト)
* スコープ
* スプレッド演算子 → [展開代入]
* [スライス](./10_array.md#スライス)
* 制御文字
* [整数] → [Int]
* [セット](./12_set.md)
* セミコロン → [;]
* [宣言](./03_declaration.md)
* 全称
  * 全称型 → [多相型](./type/quantified.md)
    * 閉じた全称型
    * 開いた全称型
  * 全称関数 → 多相関数
  * 全称量化
* 前置演算子
* 相互再帰
* 添字 → [インデックス]
* [属性]
  * [属性的部分型]

## た行

* [代数](./02_name.md)
  * [代数演算型](./type/13_algebraic.md)
  * 代数的データ型
* [代入](./02_variable.md/#代入)
* 多重
  * [多重継承](./type/07_inheritance.md/#多重継承の禁止)
  * 多重代入
  * 多重定義 → [オーバーロードの禁止]
* 多相
  * [多相型](./type/quantified.md)
  * 多相関数
* 多態 → [ポリモーフィズム]
* ダックタイピング
* [タプル](./11_tuple.md)
* 単相
  * 単相化
  * 単相型
  * 単相関数
* [遅延初期化]
* 抽出代入
* 抽象構文木 → [AST]
* 中置演算子
* [定数](./02_name.md/#定数)
  * [定数型](./type/advanced/const.md)
  * [定数式](./type/advanced/const.md)
* [定義]
* 提供属性
* [適用]
* [デコレータ](./29_decorator.md)
* デストラクタ
* 手続き → [プロシージャ](./08_procedure.md)
* [デフォルト引数](./04_function.md/#デフォルト引数default-parameters)
* 展開
  * [展開演算子]
  * [展開代入]
* [特殊形式](./../API/special.md)
* 匿名関数 → [無名関数](./20_lambda.md)
* ドット演算子(`.`) → [属性参照]
* トップ
  * トップ型 → [Structural Object]
  * トップクラス → [Object]
* [トレイト](./type/03_trait.md)

## な行

* [内包表記](./27_comprehension.md)
* ~~中置(なかおき)演算子~~ → [中置(ちゅうち)演算子]
* [名前空間]

## は行

* [配列](./10_array.md)
* [派生型](./type/variances.md/#ユーザー定義型の変性)
* [パターン(マッチ)](./26_pattern_matching.md)
* [パッケージ](./33_package_system.md)
* ハッシュマップ → [辞書](./11_dict.md)
* [パッチ](./type/07_patch.md)
* パブリック変数 → [公開変数](./19_visibility.md)
* パラメーター → [引数](./04_function.md)
* [パラメトリック多相](./type/overloading.md)
* [反変](./type/advanced/variance.md)
* 比較
  * [比較演算子]
  * [比較可能型]
* [非公開変数](./19_visibility.md)
* 標準
  * 標準出力
  * 標準入力
  * 標準ライブラリ
* [副作用](./07_side_effect.md)
* 複素数 → [Complex]
* [浮動小数点数] → [Float]
* プライベート変数 → [非公開変数]
* ブール代数 → [Bool]
* [プロシージャ](./08_procedure.md)
* [引数](./04_function.md)
* 部分型付け → [サブタイピング]
* [不変]
  * [不変オブジェクト]
  * [不変型]
  * [不変参照]
* [篩型](./type/12_refinement.md)
* [ブロック]
* 分解代入
* [変数](./02_variable.md)
* ボトム
  * ボトム型 → [{}]
  * ボトムクラス → [Never]
* [ポリモーフィズム]

## ま行

* ~~前置(まえおき)演算子~~ → 前置(ぜんち)演算子
* [マーカー型](./type/advanced/marker_trait.md)
* [無名関数](./21_lambda.md)
* ミュータブル → [可変性]
* [ムーブ]
* メソッド
* メタキャラクタ
* [モジュール](./24_module.md)
* [文字列] → [Str]
  * [文字列補間](./01_literal.md/#strリテラル)
* 戻り値

## や行

* [幽霊型](./type/advanced/phantom.md)
* 要求属性
* [要素]
* [呼び出し]

## ら行

* [ライブラリ]
* ラムダ式 → [無名関数](./20_lambda.md)
* ランク
  * [ランク2多相](./type/advanced/rank2type.md)
* [リテラル](./01_literal.md)
  * [リテラル識別子](./18_naming_rule.md/#リテラル識別子)
* [量化](./type/quantified.md)
* [レイアウト](./type/mut.md)
* [列挙型](./type/10_enum.md)
* [レコード](./12_record.md)
  * [レコード型]
  * レコード多相 → [列多相]
* [列多相]
* [ローカル変数](./19_visibility.md)

## わ行

* ワイルドカード
