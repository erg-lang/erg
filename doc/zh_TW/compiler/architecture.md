# `ergc` 的架構

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/architecture.md%26commit_hash%3Deb5b9c4946152acaecc977f47062958ce4e774a2)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/architecture.md&commit_hash=eb5b9c4946152acaecc977f47062958ce4e774a2)

## 1. 掃描 Erg 腳本 (.er) 并生成 `TokenStream` (parser/lex.rs)

src: [erg_parser/lex.rs](../../../crates/erg_parser/lex.rs)

* parser/lexer/Lexer 生成`TokenStream`(這是一個Token的迭代器，TokenStream可以通過lexer.collect()生成)
  * `Lexer` 由 `Lexer::new` 或 `Lexer::from_str` 構造，其中 `Lexer::new` 從文件或命令選項中讀取代碼
  * `Lexer` 可以作為迭代器按順序生成令牌；如果您想一次獲得 `TokenStream`，請使用 `Lexer::lex`
  * `Lexer` 輸出 `LexError` 為錯誤，但 `LexError` 沒有足夠的信息顯示自己。如果要顯示錯誤，請使用 `LexerRunner` 轉換錯誤
  * 如果你想單獨使用 `Lexer`，也可以使用 `LexerRunner`；`Lexer` 只是一個迭代器，并沒有實現 `Runnable` 特性
    * `Runnable` 由 `LexerRunner`、`ParserRunner`、`Compiler` 和 `VirtualMachine` 實現

## 2. 轉換 `TokenStream` -> `AST` (parser/parse.rs)

src: [erg_parser/parse.rs](../../../crates/erg_parser/parse.rs)

* `Parser` 和 `Lexer` 一樣，有兩個構造函數，`Parser::new` 和 `Parser::from_str`，而 `Parser::parse` 會給出 `AST`
* `AST` 是 `Vec<Expr>` 的包裝器類型

### 2.1 脫糖 `AST`

src: [erg_parser/desugar.rs](../../../crates/erg_parser/desugar.rs)

* 擴展嵌套變量 (`Desugarer::desugar_nest_vars_pattern`)
* desugar 多模式定義語法(`Desugarer::desugar_multiple_pattern_def`)

### 2.2 Reordering & Linking `AST`

src: [erg_compiler/link_ast.rs](../../../crates/erg_compiler/link_ast.rs)

* link class methods to class definitions
  * method definitions are allowed outside of the class definition file
  * current implementation is incomplete, only in the same file

## 3. 類型檢查和推斷，轉換 `AST` -> `HIR` (compiler/lower.rs)

(main) src: [erg_compiler/lower.rs](../../../crates/erg_compiler/lower.rs)

### 3.1 Name Resolution

In the current implementation it is done during type checking.

* All ASTs (including imported modules) are scanned for name resolution before type inference.
* In addition to performing cycle checking and reordering, a context is created for type inference (however, most of the information on variables registered in this context is not yet finalized).

### 3.3 Type checking & inference

src: [erg_compiler/lower.rs](../../../crates/erg_compiler/lower.rs)

* `HIR` 有每個變量的類型信息。它是用于"高級中間表示"的
* `HIR` 只保存變量的類型，但這已經足夠了。在極端情況下，這是因為 Erg 只有轉換(或運算符)應用程序的參數對象
* `ASTLower` 可以用與`Parser` 和`Lexer` 相同的方式構造
* 如果沒有錯誤發生，`ASTLowerer::lower` 將輸出 `HIR` 和 `CompileWarnings` 的元組
* `ASTLowerer`歸`Compiler`所有。與傳統結構不同，`ASTLowerer`處理代碼上下文并且不是一次性的
* 如果類型推斷的結果不完整(如果存在未知類型變量)，名稱解析時會出錯

## 4. 檢查副作用(compiler/effectcheck.rs)

src: [erg_compiler/effectcheck.rs](../../../crates/erg_compiler/effectcheck.rs)

## 4. 檢查所有權(compiler/memcheck.rs)

## 5. 從`HIR`(compiler/codegen.rs)生成字節碼(`CodeObj`)

src: [erg_compiler/ownercheck.rs](../../../crates/erg_compiler/ownercheck.rs)

* 根據表達式的類型信息，將執行量化子程序的名稱解析

## 6. Optimize `HIR`

src: [erg_compiler/optimize.rs](../../../crates/erg_compiler/optimize.rs)

* Eliminate dead code (unused variables, imports, etc.)
* 字節碼是基于堆棧的，而 LLVM IR 是基于寄存器的
  這個轉換過程會多出幾層中間過程。

## 7. Link

src: [erg_compiler/link_hir.rs](../../../crates/erg_compiler/link_hir.rs)

* Load all modules, resolve dependencies, and combine into a single HIR

## 8. Desugar `HIR`

src: [erg_compiler/desugar_hir.rs](../../../crates/erg_compiler/desugar_hir.rs)

Convert parts that are not consistent with Python syntax

* Convert class member variables to functions

## 8. Generate Bytecode (`CodeObj`) from `HIR`

src: [erg_compiler/codegen.rs](../../../crates/erg_compiler/codegen.rs)
