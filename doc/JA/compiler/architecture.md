# ergc のアーキテクチャ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/architecture.md%26commit_hash%3Db9538ca627ab5459bae79eec48c1d676268875ab)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/architecture.md&commit_hash=b9538ca627ab5459bae79eec48c1d676268875ab)

## 1. Erg スクリプト (.er) をスキャンし、`TokenStream` を生成する

src: [erg_parser/lex.rs](../../../crates/erg_parser/lex.rs)

* parser/lexer/Lexer が `TokenStream` を生成する (これは `Token` のイテレータである。`TokenStream` は `Lexer::collect()` によって生成できる)
  * [`Lexer`](./phases/01_lex.md) は `Lexer::new` または `Lexer::from_str` から構築される。`Lexer::new` はファイルまたはコマンド オプションからコードを読み取る。
  * `Lexer` はイテレータとしてトークンを順次生成できるので、一度に `TokenStream` を取得したい場合は `Lexer::lex` を使う。
  * `Lexer` は `LexError` をエラーとして出力するが、`LexError` 自体には表示するだけの情報がない。エラーを表示したい場合は、`LexerRunner` を使用してエラーを変換する。
  * `Lexer` を単体で使用する場合は、代わりに`LexerRunner` を使用します。`Lexer` は単なるイテレータであり、`Runnable` トレイトを実装していない。
    * `Runnable` は、 `LexerRunner` 、 `ParserRunner` 、 `Compiler` 、および `DummyVM` に実装されている。

## 2. `TokenStream` -> `AST`

src: [erg_parser/parse.rs](../../../crates/erg_parser/parse.rs)

* [`Parser`](./phases/02_parse.md) は `Lexer` と同様に `Parser::new` と `Parser::from_str` の 2 つのコンストラクタを持ち、`Parser::parse` は `AST` を返す。
* `AST`は`Vec<Expr>`のラッパー型で、「抽象構文木」を表す。

### 2.1 `AST`の脱糖

src: [erg_parser/desugar.rs](../../../crates/erg_parser/desugar.rs)

* [`Desugarer`](./phases/03_desugar.md)
* パターンマッチを単一の変数代入列へ変換 (`Desugarer::desugar_nest_vars_pattern`)
* 複数パターン定義構文をmatchへ変換 (`Desugarer::desugar_multiple_pattern_def`)

### 2.2 `AST`の並び替え・結合

src: [erg_compiler/link_ast.rs](../../../crates/erg_compiler/link_ast.rs)

* クラスメソッドをクラス定義に結合する
  * メソッド定義は定義ファイル外でも可能となっている
  * 現在の実装は不完全、同一ファイル内のみ

## 3. `AST` -> `HIR`

(主要な)ソースコード: [erg_compiler/lower.rs](../../../crates/erg_compiler/lower.rs)

### 3.1 名前解決

現在の実装では型チェック中に行われる

* 型推論の前に全てのAST(importされたモジュール含む)を走査し、名前解決を行う
* 定数の循環検査や並び替えなどが行われるほか、型推論のためのContextが作成される(ただし、このContextに登録された変数の情報ははまだ殆どが未確定)

### 3.2 import解決

* `import`に出会うと、新しくスレッドを作成して検査を行う
* `JoinHandle`は`SharedCompilerResource`に格納され、該当モジュールが必要になったときにjoinされる
* 使用されなかったモジュールはjoinされないことがあるが、現在のところはそのようなモジュールも全て検査される

### 3.3 型チェックと推論

ソースコード: [erg_compiler/lower.rs](../../../crates/erg_compiler/lower.rs)

* `HIR` は、すべての変数の型情報を持っており、「高レベルの中間表現」を表す。
* `ASTLowerer` は Parser や Lexer と同じように構築できる。
* `ASTLowerer::lower` は、`CompleteArtifact`か`IncompleteArtifact`を出力する。両者とも`HIR` と `LowerWarnings` を持ち、後者は`LowerErrors`も持つ。
* `ASTLowerer` は `Compiler` によって所有されている。`ASTLowerer` は`Lexer`や`Parser`とは異なり、文脈を保持し、1 回限りの使い捨てではない。
* 型推論の結果が不完全な場合(未知の型変数がある場合)、名前解決時にエラーが発生する。

## 4. 副作用のチェック

ソースコード: [erg_compiler/effectcheck.rs](../../../crates/erg_compiler/effectcheck.rs)

## 5. 所有権の確認

ソースコード: [erg_compiler/ownercheck.rs](../../../crates/erg_compiler/ownercheck.rs)

## 6. 最適化

ソースコード: [erg_compiler/optimize.rs](../../../crates/erg_compiler/optimize.rs)

* 不要な変数(import含む)を削除する

## 7. リンク

ソースコード: [erg_compiler/link_hir.rs](../../../crates/erg_compiler/link_hir.rs)

* 全てのモジュールを読み込み、依存関係を解決し、単一のHIRに結合する

## 8. `HIR`の脱糖

ソースコード: [erg_compiler/desugar_hir.rs](../../../crates/erg_compiler/desugar_hir.rs)

* Pythonの文法と整合しない部分を変換する
  * クラスのメンバ変数を関数に変換

## 8. `HIR` からバイトコード (`CodeObj`) を生成

ソースコード: [erg_compiler/codegen.rs](../../../crates/erg_compiler/codegen.rs)
