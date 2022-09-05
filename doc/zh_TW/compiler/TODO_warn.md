# 警告(尚未實現)

* `t = {(record type)}` => `T = {(record type)}`?(只有定義為常量的類型才能用于類型說明)
* `{I: Int | ...}!` => `{I: Int! | ...}`
* for/while 塊中的`return x`(`x != ()`) => `f::return`(外部塊)？