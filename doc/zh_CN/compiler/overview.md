# `erg` 概览

我们将介绍每一层的功能以及特别重要的功能和方法。

## 1. 词法分析

* `Lexer` 进行词法分析。 `Lexer::next`(`Lexer`被实现为一个迭代器)负责词法分析的主要逻辑。 `Token` 作为解析的结果输出。

## 2. 解析

* `Parser` 进行解析。特别重要的是`Parser::parse_expr`。作为解析的结果，输出作为`ast::Expr`的集合的`AST`。

## 3. 脱糖

* 脱糖由 `Desugarer` 完成。 `AST` 将被输出。

## 4. 类型检查/类型推断

* `ASTLowerer` 进行打字。类型检查主要由 `Context` 完成。尤其重要的是 `Context::supertype_of`(确定子类型关系)、`Context::unify/sub_unify`(统一/半统一类型变量)、`Context::init_builtin_*`(定义内置 API)。 `HIR` 作为分析结果输出。

## 5. 副作用检查

* `SideEffectChecker` 可以。

## 6. 所有权检查

* `OwnershipChecker` 可以。

## 7. 字节码生成

* `CodeGenerator` 将 `HIR` 转换为 `CodeObj`。 `CodeObj` 保存字节码和执行配置。特别重要的是`CodeGenerator::compile_expr`。

---

* 以上所有的处理都是由`Compiler`作为一个门面组合起来的。
* 当然 Python 会执行生成的字节码，称为 `DummyVM`。