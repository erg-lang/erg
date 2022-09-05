# 錯誤恢復建議(尚未實現)

*  `1 or 2`, `1 and 2` => `{1, 2}`?
* `U = Inherit T` => 非類類型不能被繼承，或者`U = Class T`？
* `Int and Str` => 不允許多重繼承，或者`Int or Str`？
* `: [1, 2]` => `: {1, 2}`？
* `: [Int, 2]` => `: [Int; 2]`？
* `[Int; Str]` => `(Int, Str)`(Tuple) 還是 `[Int: Str]`(Dict)？
* `{x: Int}` => `{x = Int}`？
* `{x = Int}!` => `{x = Int!}`？
* `ref! immut_expr` => `ref! !immut_expr`?