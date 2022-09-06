# Error recovery suggestions (not implemented yet)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/TODO_recov_suggest.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/TODO_recov_suggest.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

* `1 or 2`, `1 and 2` => `{1, 2}`?
* `U = Inherit T` => Non-class type cannot be inherited, or `U = Class T`?
* `Int and Str` => Multiple inheritance is not allowed, or `Int or Str`?
* `: [1, 2]` => `: {1, 2}`?
* `: [Int, 2]` => `: [Int; 2]`?
* `[Int; Str]` => `(Int, Str)`(Tuple) or `[Int: Str]`(Dict)?
* `{x: Int}` => `{x = Int}`?
* `{x = Int}!` => `{x = Int!}`?
* `ref! immut_expr` => `ref! !immut_expr`?
