# `erg`の概観

各レイヤーの働きと特に重要な関数、メソッドを紹介します。

## 1. 字句解析

* `Lexer`が字句解析を行います。`Lexer::next`(`Lexer`はイテレータとして実装されています)が字句解析のメインロジックを担います。解析の結果として`Token`が出力されます。

## 2. 構文解析

* `Parser`が構文解析を行います。特に重要なのは`Parser::parse_expr`です。解析の結果として`ast::Expr`の集まりである`AST`が出力されます。

## 3. 脱糖

* `Desugarer`が脱糖を行います。`AST`が出力されます。

## 4. 型検査/型推論

* `ASTLowerer`が型付けを行います。型検査は主に`Context`によって行われます。特に重要なのは`Context::supertype_of`(部分型関係を判定する), `Context::unify/sub_unify`(型変数の単一化/半単一化を行う), `Context::init_builtin_*`(組み込みAPIを定義する)です。解析の結果として`HIR`が出力されます。

## 5. 副作用チェック

* `SideEffectChecker`が行います。

## 6. 所有権チェック

* `OwnershipChecker`が行います。

## 7. バイトコード生成

* `CodeGenerator`が`HIR`を`CodeObj`に変換します。`CodeObj`はバイトコードと実行設定を保持します。特に重要なのは`CodeGenerator::compile_expr`です。

---

* 以上のすべての処理は`Compiler`がファサードとしてまとめます。
* 生成されたバイトコードの実行は、もちろんPythonが行いますが、これを呼ぶのが`DummyVM`です。
