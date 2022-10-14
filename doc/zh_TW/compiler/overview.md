# `erg` 概覽

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/overview.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/overview.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

我們將介紹每一層的功能以及特別重要的功能和方法

## 1. 詞法分析

* `Lexer` 進行詞法分析。`Lexer::next`(`Lexer`被實現為一個迭代器)負責詞法分析的主要邏輯。`Token` 作為解析的結果輸出

## 2. 解析

* `Parser` 進行解析。特別重要的是`Parser::parse_expr`。作為解析的結果，輸出作為`ast::Expr`的集合的`AST`

## 3. 脫糖

* 脫糖由 `Desugarer` 完成。`AST` 將被輸出

## 4. 類型檢查/類型推斷

* `ASTLowerer` 進行打字。類型檢查主要由 `Context` 完成。尤其重要的是 `Context::supertype_of`(確定子類型關系)、`Context::unify/sub_unify`(統一/半統一類型變量)、`Context::init_builtin_*`(定義內置 API)。`HIR` 作為分析結果輸出

## 5. 副作用檢查

* `SideEffectChecker` 可以

## 6. 所有權檢查

* `OwnershipChecker` 可以

## 7. 字節碼生成

* `CodeGenerator` 將 `HIR` 轉換為 `CodeObj`。`CodeObj` 保存字節碼和執行配置。特別重要的是`CodeGenerator::compile_expr`

---

* 以上所有的處理都是由`Compiler`作為一個門面組合起來的
* 當然 Python 會執行生成的字節碼，稱為 `DummyVM`。