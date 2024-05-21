# Type checking

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/05_type_check.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/05_type_check.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

Erg's type analysis is part of the lowering phase, which converts the AST to the HIR. The HIR is a slightly more compact syntax tree (intermediate representation) than the AST, with types explicitly specified for all expressions.
The `ASTLowerer` performs the lowering, and the `Context` structure executes the type analysis.

Erg's type analysis has two aspects: type checking and type inference. Both are executed simultaneously.
In type checking, it refers to type specifications and the type environment to check if terms are used according to the rules. In type inference, it issues type variables for unspecified types and unifies them by referring to the type environment.
