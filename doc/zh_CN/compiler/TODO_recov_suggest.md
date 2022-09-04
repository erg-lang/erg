# 错误恢复建议（尚未实现）

*  `1 or 2`, `1 and 2` => `{1, 2}`?
* `U = Inherit T` => 非类类型不能被继承，或者`U = Class T`？
* `Int and Str` => 不允许多重继承，或者`Int or Str`？
* `: [1, 2]` => `: {1, 2}`？
* `: [Int, 2]` => `: [Int; 2]`？
* `[Int; Str]` => `(Int, Str)`(Tuple) 还是 `[Int: Str]`(Dict)？
* `{x: Int}` => `{x = Int}`？
* `{x = Int}!` => `{x = Int!}`？
* `ref! immut_expr` => `ref! !immut_expr`?