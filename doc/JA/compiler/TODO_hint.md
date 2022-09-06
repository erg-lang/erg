# Hint (not implemented)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/TODO_hint.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/TODO_hint.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

* `x is not defined` (x was deleted by `Del`) => `hint: deleted in line X`
* patch method duplication: "hint: Specify patch (like `T.foo(1)`) or delete either `.foo` using `Del`"
