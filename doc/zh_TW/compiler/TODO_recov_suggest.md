# Error recovery suggestions (not implemented yet)

* `1 or 2`, `1 and 2` => `{1, 2}`?
* `U = Inherit T` => Non-class type cannot be inherited, or `U = Class T`?
* `Int and Str` => Multiple inheritance is not allowed, or `Int or Str`?
* `: [1, 2]` => `: {1, 2}`?
* `: [Int, 2]` => `: [Int; 2]`?
* `[Int; Str]` => `(Int, Str)` (Tuple) or `[Int: Str]` (Dict)?
* `{x: Int}` => `{x = Int}`?
* `{x = Int}!` => `{x = Int!}`?
* `ref! immut_expr` => `ref!!immut_expr`?
