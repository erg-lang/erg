# ergc のアーキテクチャ

## 1. Erg スクリプト (.er) をスキャンし、`TokenStream` (parser/lex.rs) を生成します

* parser/lexer/Lexer は `TokenStream` を生成します (これは Token のイテレータです。TokenStream は lexer.collect() によって生成できます)
  * `Lexer` は `Lexer::new` または `Lexer::from_str` から構築されます。`Lexer::new` はファイルまたはコマンド オプションからコードを読み取ります。
  ※ Lexer はイテレータとしてトークンを順次生成できるので、一度に TokenStream を取得したい場合は Lexer::lex を使う。
  * `Lexer` は `LexError` をエラーとして出力しますが、`LexError` 自体には表示するだけの情報がありません。エラーを表示したい場合は、`LexerRunner` を使用してエラーを変換します。
  * `Lexer` をスタンドアロンとして使用する場合は、`LexerRunner` も使用できます。`Lexer` は単なるイテレータであり、`Runnable` トレイトを実装していません。
    * Runnable は、 LexerRunner 、 ParserRunner 、 Compiler 、および VirtualMachine によって実装されます。

## 2. `TokenStream` を変換 -> `AST` (parser/parse.rs)

* `Parser` は `Lexer` と同様に `Parser::new` と `Parser::from_str` の 2 つのコンストラクタを持ち、`Parser::parse` は `AST` を返します。
※`AST`は`Vec<Expr>`のラッパー型で、「抽象構文木」用です。

### 2.5 `AST`の脱糖

* ネストされた変数を展開 (`Desugarer::desugar_nest_vars_pattern`)
* desugar 複数パターン定義構文 (`Desugarer::desugar_multiple_pattern_def`)

## 3. 型チェックと推論、 `AST` -> `HIR` を変換 (compiler/lower.rs)

* `HIR` は、すべての変数の型情報を持っています. これは、「高レベルの中間表現」用です.
* `HIR` は変数の型しか保持していないが、それで十分. 極端な場合、これは Erg が変換 (または演算子) のアプリケーションしか持たないためです. 変換の型がわかれば、変数の型もわかっています.引数のオブジェクト。
※ ASTLowerer は Parser や Lexer と同じように構築できます。
* `ASTLowerer::lower` は、エラーが発生しなければ、`HIR` と `CompileWarnings` のタプルを出力します。
* `ASTLowerer` は `Compiler` によって所有されています. `ASTLowerer` は従来の構造体とは異なり、コード コンテキストを処理し、1 回限りの使い捨てではありません。
※型推論の結果が不完全な場合(未知の型変数がある場合)、名前解決時にエラーが発生します。

## 4. 副作用のチェック (compiler/effectcheck.rs)

## 4. 所有権の確認 (compiler/memcheck.rs)

## 5. `HIR` からバイトコード (`CodeObj`) を生成 (compiler/codegen.rs)

※式の型情報から、量化されたサブルーチンの名前解決を行います。

## (6. (今後の予定) バイトコード変換 -> LLVM IR)

* バイトコードはスタックベースですが、LLVM IR はレジスタベースです。
  この変換プロセスには、さらにいくつかの中間プロセスのレイヤーがあります。