# 索引

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/indexes.md%26commit_hash%3Dd8472ec748aac5371571da81a161255fe60679b7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/indexes.md&commit_hash=d8472ec748aac5371571da81a161255fe60679b7)

この索引にないAPIについては[こちら](../API/index.md)を参照してください。

用語の意味については[こちら](../terms.md)を参照。

## 記号

* ! → [side&nbsp;effect](./07_side_effect.md)
  * !-type → [mutable&nbsp;type](./type/18_mut.md)
* ? → [error&nbsp;handling](./30_error_handling.md)
* &#35; → [Str](./00_basic.md/#comment)
* $ → [shared](./type/advanced/shared.md)
* %
* &
  * &&
* [&prime;&nbsp;(single&nbsp;quote)](./20_naming_rule.md)
* [&quot;&nbsp;(double&nbsp;quote)](./01_literal.md)
* &lpar;&rpar; → [Tuple](./11_tuple.md)
* &ast;
  * &ast; → [*-less&nbsp;multiplication](./01_literal.md/#less-multiplication)
* &plus; (前置) → [operator](./06_operator.md)
  * &plus;_ → &plus; (前置)
* &plus; (中置) → [operator](./06_operator.md)
* &plus; (中置) → [Trait](./type/03_trait.md)
* ,
* &minus; (前置)
  * &minus;_ → &minus; (前置)
* &minus; (中置) → [operator](./06_operator.md)
* &minus; (中置) → [Trait](./type/03_trait.md)
  * &minus;> → [anonymous&nbsp;function](./21_lambda.md)
* . → [Visibility](./19_visibility.md)
  * [...&nbsp;assignment](./28_spread_syntax.md)
  * [...&nbsp;Extract&nbsp;assignment](./28_spread_syntax.md)
  * [...&nbsp;function](./04_function.md)
* /
* :
  * : → [Colon&nbsp;application&nbsp;style](./04_function.md)
  * : → [Declaration](./03_declaration.md)
  * : → [Keyword&nbsp;Arguments](./04_function.md)
  * :: → [visibility](./19_visibility.md)
  * := → [default&nbsp;parameters](./04_function.md)
* ;
* &lt;
  * &lt;: → [Subtype&nbsp;specification](./type/02_basic.md)
  * &lt;&lt;
  * &lt;=
* = → [Variable](./19_visibility.md)
  * ==
  * => → [procedure](./08_procedure.md)
* &gt;
  * &gt;&gt;
  * &gt;=
* @ → [decorator](./29_decorator.md)
* [] → [Array](./10_array.md)
* \ → [Indention](./00_basic.md)
* \ → [Str](./01_literal.md)
* ^
  * ^^
* _ → [Type&nbsp;erasure](./type/advanced/erasure.md)
  * &#95;+&#95; → &plus; (infix)
  * &#95;-&#95; → &minus; (infix)
* [``&nbsp;(back&nbsp;quote)](./22_subroutine.md)
* {}
  * [{} type](./type/01_type_system.md)
* {:}
* {=} → [Type&nbsp;System](./type/01_type_system.md)
  * [{=}&nbsp;type](./13_record.md)
* |
  * || → [Type variable list](./type/advanced/)
* ~

## アルファベット

### A

* [Add]
* [alias](type/02_basic.md)
* [Aliasing](./type/02_basic.md)
* [All&nbsp;symmetric&nbsp;types](./type/15_quantified.md)
* [algebraic&nbsp;type](./type/13_algebraic.md)
* [And]
* [and]
* [anonymous&nbsp;function](./21_lambda.md)
* [Anonymous&nbsp;polycorrelation&nbsp;coefficient](./21_lambda.md)
* anonymous type → [Type&nbsp;System](./type/01_type_system.md)
* [Array](./10_array.md)
* [assert]
* [Attach](./29_decorator.md)
* [attribute](type/09_attributive.md)
* [Attribute&nbsp;definitions](./type/02_basic.md)
* [Attribute&nbsp;Type](./type/09_attributive.md)

### B

* [Bool, Boolean](./01_literal.md)
* [Boolean&nbsp;Object](./01_literal.md)
* [borrow](./18_ownership.md)

### C

* [Cast](./type/17_type_casting.md)
* [Comments](./00_basic.md)
* [Complex&nbsp;Object](./01_literal.md)
* [Compile-time&nbsp;functions](./04_function.md)
* [circular&nbsp;references](./18_ownership.md)
* [Class](./type/04_class.md)
* [Class&nbsp;Relationship](./type/04_class.md)
* [Class&nbsp;upcasting](./type/16_subtyping.md)
* [Colon&nbsp;application&nbsp;style](./04_function.md)
* [Closure](./23_closure.md)
* [Compound Literals](./01_literal.md)
* [Complement](./type/13_algebraic.md)
* [Comprehension](./27_comprehension.md)
* [constant](./17_mutability.md)
* [Constants](./02_name.md)
* [Context](./30_error_handling.md)

### D

* [Data&nbsp;type](./type/01_type_system.md)
* [Declaration](./03_declaration.md)
* [decorator](./29_decorator.md)
* [Default&nbsp;parameters](./04_function.md)
* [Del](./02_name.md)
* [Dependent&nbsp;Type](./type/14_dependent.md)
* [Deconstructing&nbsp;a&nbsp;record](13_record.md)
* Deprecated
* [Dict](./12_dict.md)
* [Diff](./type/13_algebraic.md)
* [Difference&nbsp;from&nbsp;Data&nbsp;Class](./type/04_class.md)
* [Difference&nbsp;from&nbsp;structural&nbsp;types](type/04_class.md)
* distinct
* [Downcasting](./type/17_type_casting.md)

### E

* [Empty&nbsp;Record](./13_record.md)
* [Enum&nbsp;Class](./type/04_class.md)
* [Enum&nbsp;type](./type/11_enum.md)
* [Enumerated,&nbsp;Interval&nbsp;and&nbsp;Refinement&nbsp;Types](./type/12_refinement.md)
* [error&nbsp;handling](./30_error_handling.md)
* [Existential&nbsp;type](./type/advanced/existential.md)
* [Exponential&nbsp;Literal](./01_literal.md)
* [Extract&nbsp;assignment](./28_spread_syntax.md)

### F

* False → [Boolean Object](./01_literal.md)
* [Float&sbsp;Object](./01_literal.md)
* [for](./05_builtin_funcs.md)
* [For-All&nbsp;Patch](./type/07_patch.md)
* [freeze](./18_ownership.md)
* [Function](./04_function.md)
* [Function&nbsp;definition&nbsp;with&nbsp;multiple patterns](./04_function.md)

### G

* [GADTs(Generalized&nbsp;Algebraic&nbsp;Data&nbsp;Types)](./type/advanced/GADTs.md)
* [Generator](./34_generator.md)
* [Glue&nbsp;Patch](./type/07_patch.md)

### H

### I

* [id](./09_builtin_procs.md)
* [if](./05_builtin_funcs.md)
* [import](./33_package_system.md)
* [impl](./29_decorator.md)
* [in]
* [Indention](./00_basic.md)
* [Instant&nbsp;Block](./13_record.md)
* [Instance&nbsp;and&nbsp;class&nbsp;attributes](./type/04_class.md)
* [Implementing&nbsp;and&nbsp;resolving&nbsp;duplicate&nbsp;traits&nbsp;in&nbsp;the&nbsp;API](type/03_trait.md)
* [inheritable](./29_decorator.md)
* [inheritance](./type/05_inheritance.md)
* [Inheritance&nbsp;of&nbsp;Enumerated&nbsp;Classes](./type/05_inheritance.md)
* [Int](./01_literal.md)
* [Integration&nbsp;with&nbsp;Python](./32_integration_with_Python.md)
* [Interval&nbsp;Type](./type/10_interval.md)
* [Intersection](./type/13_algebraic.md)
* [Iterator](./16_iterator.md)

### J

### K

* [Keyword&nbsp;arguments](./04_function.md)
* [Kind](./type/advanced/kind.md)

### L

* lambda → [anonymous&nbsp;function](./21_lambda.md)
* let-polymorphism → [rank&nbsp;1&nbsp;polymorphism]
* [Literal&nbsp;Identifiers](./20_naming_rule.md)
* log → [side&nbsp;effect](./07_side_effect.md)

### M

* [match]
* [Marker&nbsp;Trait](./type/advanced/marker_trait.md)
* [Method](./07_side_effect.md)
* Modifier → [decorator](./29_decorator.md)
* [module](./24_module.md)
* [Multiple&nbsp;Inheritance](type/05_inheritance.md)
* [Multi-layer&nbsp;(multi-level)&nbsp;Inheritance](type/05_inheritance.md)
* [Mutable&nbsp;Type](./type/18_mut.md)
* [Mutable&nbsp;Structure&nbsp;Type](./type/advanced/mut_struct.md)
* [Mutability](./17_mutability.md)

### N

* [Nat](./01_literal.md)
* [Never]
* [New&nbsp;type](./type/advanced/newtype.md)
* [Heterogeneous&nbsp;Dict](./12_dict.md)
* None → [None&nbsp;Object]
* [None&nbsp;Object]
* Nominal&nbsp;Subtyping → [Class](./type/04_class.md)
* [Not]
* [not]

### O

* [Object](./25_object_system.md)
* [Option]
* [Or]
* [or]
* [Ord]
* [ownership&nbsp;system](./18_ownership.md)
* [Overloading](./type/advanced/overloading.md)
* [Overriding](./type/05_inheritance.md)
* [Override&nbsp;in&nbsp;Trait](./type/03_trait.md)

### P

* [Panic](./30_error_handling.md)
* [Patch](./type/07_patch.md)
* [Pattern&nbsp;match](./26_pattern_matching.md)
* [Phantom&nbsp;class](./type/advanced/phantom.md)
* [pipeline&nbsp;operator](./31_pipeline.md)
* [Predicate](./type/19_bound.md)
* [print!]
* [Procedures](./08_procedure.md)
* [Projection&nbsp;Type](./type/advanced/projection.md)
* Python → [Integration&nbsp;with&nbsp;Python](./32_integration_with_Python.md)

### Q

* [Quantified&nbsp;Type](./type/15_quantified.md)
* [Quantified&nbsp;Dependent&nbsp;Type](./type/advanced/quantified_dependent.md)
* [Quantified&nbsp;Types&nbsp;and&nbsp;Dependent&nbsp;Types](./type/15_quantified.md)

### R

* [Range&nbsp;Object](./01_literal.md)
* [ref]
* [ref!]
* [Record](./13_record.md)
* [Record&nbsp;Type&nbsp;Composite](./type/09_attributive.mda12_refinement.md)
* [Recursive&nbsp;functions](./04_function.md)
* [Refinement&nbsp;pattern](./type/12_refinement.md)
* [Refinement&nbsp;Type](./type/12_refinement.md)
* [replication](./18_ownership.md)
* [Replacing&nbsp;Traits](./type/05_inheritance.md)
* Result → [error&nbsp;handling](./30_error_handling.md)
* [Rewriting&nbsp;Inherited&nbsp;Attributes](./type/05_inheritance.md)
* rootobj

### S

* [Script](./00_basic.md)
* [Selecting&nbsp;Patches](./type/07_patch.md)
* self
* [Self](./type/advanced/special.md)
* [Shared&nbsp;Reference](./type/advanced/shared.md)
* [side-effect](./07_side_effect.md)
* [Smart&nbsp;Cast](./type/12_refinement.md)
* [Spread&nbsp;assignment](./28_spread_syntax.md)
* [special&nbsp;type&nbsp;variables](./type/advanced/special.md)
* [Stack&nbsp;trace](30_error_handling.md)
* [Structure&nbsp;type](./type/01_type_system.md)
* [Structural&nbsp;Patch](./type/07_patch.md)
* [Structural&nbsp;Trait](./type/03_trait.md)
* [Structural&nbsp;Subtyping](./type/01_type_system.md)
* [Structural&nbsp;types&nbsp;and&nbsp;class&nbsp;type&nbsp;relationships](./type/16_subtyping.md)
* [Str](./01_literal.md)
* [Subtyping](./type/16_subtyping.md)
* [Subtyping&nbsp;of&nbsp;subroutines](./type/16_subtyping.md)
* [Subtype&nbsp;specification](./type/02_basic.md)
* [Subtyping&nbsp;of&nbsp;Polymorphic&nbsp;Function Types](./type/15_quantified.md)
* [Subroutine&nbsp;Signatures](./22_subroutine.md)

### T

* [Test](./29_decorator.md)
* [Traits](./type/03_trait.md)
* [Trait&nbsp;inclusion](./type/03_trait.md)
* True → [Boolean&nbsp;Object](./01_literal.md)
* [True&nbsp;Algebraic&nbsp;type](./type/13_algebraic.md)
* [Type]
* [type](./15_type.md)
* [Type&nbsp;arguments&nbsp;in&nbsp;method&nbsp;definitions](./type/15_quantified.md)
* [Type&nbsp;Bound](./type/19_bound.md)
* [Type&nbsp;Definitions](./type/01_type_system.md)
* [Type&nbsp;erasure](./type/advanced/erasure.md)
* [Type&nbsp;Inference&nbsp;System](./type/01_type_system.md)
* [Type&nbsp;specification](./type/02_basic.md)
* [Type&nbsp;System](./type/01_type_system.md)
* [Type&nbsp;Widening](./type/advanced/widening.md)
* [Tuple](./11_tuple.md)

### U

* [union](type/13_algebraic.md)
* [Unit](./11_tuple.md)
* [Upcasting](type/17_type_casting.md)
* [Usage&nbsp;of&nbsp;Inheritance](./type/05_inheritance.md)

### V

* [Value&nbsp;Type](./type/08_value.md)
* [Variable](./02_name.md)
* [variable-length&nbsp;arguments](./04_function.md)

### W

* [while]

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
