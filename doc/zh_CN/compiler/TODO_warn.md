# 警告（尚未实现）

* `t = {(record type)}` => `T = {(record type)}`?（只有定义为常量的类型才能用于类型说明）
* `{I: Int | ...}!` => `{I: Int! | ...}`
* for/while 块中的`return x`(`x != ()`) => `f::return`（外部块）？