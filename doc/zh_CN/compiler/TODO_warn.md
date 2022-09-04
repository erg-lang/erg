# warnings (not implemented yet)

* `t = {(record type)}` => `T = {(record type)}`? (only types defined as constants can be used for type specification)
* `{I: Int | ...}!` => `{I: Int! | ...}`
* `return x`(`x != ()`) in for/while block => `f::return` (outer block)?
