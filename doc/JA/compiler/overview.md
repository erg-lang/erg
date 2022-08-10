# `erg`の概観

各レイヤーの働きと特に重要な関数、メソッドを紹介します。

## 1. 字句解析

* `Lexer`が字句解析を行います。`Lexer::next`が字句解析のメインロジックを担います。解析の結果として`Token`が出力されます。

## 2. 構文解析

* `Parser`が構文解析を行います。特に重要なのは`Parser::parse_expr`です。解析の結果として`ast::Expr`の集まりである`AST`が出力されます。

## 3. 型チェック

* `ASTLowerer`がASTをHIRに変換して型付けを行います。型チェックは主に`SymbolTable`によって行われます。特に重要なのは`SymbolTable::supertype_of`(部分型関係を判定する), `SymbolTable::unify`(型変数の単一化を行う), `SymbolTable::init_builtin_*`(組み込みAPIを定義する)です。解析の結果として`HIR`が出力されます。

## 4. 副作用チェック

## 5. 所有権チェック

## 6. バイトコード生成

* `Compiler`が`HIR`を`CodeObj`に変換します。`CodeObj`はバイトコードと実行設定を保持します。特に重要なのは`Compiler::compile_expr`です。
