# Type checking

Erg's type analysis is part of the lowering phase, which converts the AST to the HIR. The HIR is a slightly more compact syntax tree (intermediate representation) than the AST, with types explicitly specified for all expressions.
The `ASTLowerer` performs the lowering, and the `Context` structure executes the type analysis.

Erg's type analysis has two aspects: type checking and type inference. Both are executed simultaneously.
In type checking, it refers to type specifications and the type environment to check if terms are used according to the rules. In type inference, it issues type variables for unspecified types and unifies them by referring to the type environment.
