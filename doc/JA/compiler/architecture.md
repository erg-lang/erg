# ergc のアーキテクチャ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/architecture.md%26commit_hash%3Da711efa99b325ba1012f6897e7b0e2bdb947d8a1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/architecture.md&commit_hash=a711efa99b325ba1012f6897e7b0e2bdb947d8a1)

## 1. Erg スクリプト (.er) をスキャンし、`TokenStream` (parser/lex.rs) を生成する

* parser/lexer/Lexer が `TokenStream` を生成する (これは `Token` のイテレータである。`TokenStream` は `Lexer::collect()` によって生成できる)
  * `Lexer` は `Lexer::new` または `Lexer::from_str` から構築される。`Lexer::new` はファイルまたはコマンド オプションからコードを読み取る。
  * `Lexer` はイテレータとしてトークンを順次生成できるので、一度に `TokenStream` を取得したい場合は `Lexer::lex` を使う。
  * `Lexer` は `LexError` をエラーとして出力するが、`LexError` 自体には表示するだけの情報がない。エラーを表示したい場合は、`LexerRunner` を使用してエラーを変換する。
  * `Lexer` を単体で使用する場合は、代わりに`LexerRunner` を使用します。`Lexer` は単なるイテレータであり、`Runnable` トレイトを実装していない。
    * `Runnable` は、 `LexerRunner` 、 `ParserRunner` 、 `Compiler` 、および `DummyVM` に実装されている。

## 2. `TokenStream` -> `AST` (parser/parse.rs)

* `Parser` は `Lexer` と同様に `Parser::new` と `Parser::from_str` の 2 つのコンストラクタを持ち、`Parser::parse` は `AST` を返す。
* `AST`は`Vec<Expr>`のラッパー型で、「抽象構文木」を表す。

### 2.1 `AST`の脱糖

* パターンマッチを単一の変数代入列へ変換 (`Desugarer::desugar_nest_vars_pattern`)
* 複数パターン定義構文をmatchへ変換 (`Desugarer::desugar_multiple_pattern_def`)

## 3. `AST` -> `HIR`  (compiler/lower.rs)

## 3.1 名前解決

* 型推論の前に全てのAST(importされたモジュール含む)を走査し、名前解決を行う
* 定数の循環検査や並び替えなどが行われるほか、型推論のためのContextが作成される(ただし、このContextに登録された変数の情報ははまだ殆どが未確定)

### 3.2 型チェックと推論 (compiler/lower.rs)

* `HIR` は、すべての変数の型情報を持っており、「高レベルの中間表現」を表す。
* `ASTLowerer` は Parser や Lexer と同じように構築できる。
* `ASTLowerer::lower` は、エラーが発生しなければ、`HIR` と `CompileWarnings` のタプルを出力する。
* `ASTLowerer` は `Compiler` によって所有されている。 `ASTLowerer` は従来の構造体とは異なり、文脈を保持し、1 回限りの使い捨てではない。
* 型推論の結果が不完全な場合(未知の型変数がある場合)、名前解決時にエラーが発生する。

## 4. 副作用のチェック (compiler/effectcheck.rs)

## 4. 所有権の確認 (compiler/memcheck.rs)

## 5. `HIR`の脱糖 (compiler/desugar_hir.rs)

* Pythonの文法と整合しない部分を変換する

* クラスのメンバ変数を関数に変換

## 6. `HIR` からバイトコード (`CodeObj`) を生成 (compiler/codegen.rs)

## (7. (今後の予定) バイトコード -> LLVM IR)

* バイトコードはスタックベースだが、LLVM IR はレジスタベースである。
  この変換プロセスには、さらにいくつかの中間プロセスのレイヤーが必要となる。
