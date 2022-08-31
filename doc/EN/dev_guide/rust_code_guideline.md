# Guidelines for Rust code

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/rust_code_guideline.md%26commit_hash%3Dfc7a25a8d86c208fb07beb70ccc19e4722c759d3)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/rust_code_guideline.md&commit_hash=fc7a25a8d86c208fb07beb70ccc19e4722c759d3)

## Local rules

* Use `log!` for debugging output (use `println!` etc. for output processing required for release).
* Unused or internal variables and methods (private and used only for specific functions) should be prefixed with `_`. To avoid conflicts with reserved words, add one trailing `_`.

## Encouraged code

* Define domain-specific Enums instead of numeric enumerations or bools.
* Minimize access modifiers. Use `pub(mod)` or `pub(crate)` in preference even when publishing.
* Explicitly convert iterable objects in for expressions to iterators (`for i in x.iter()` instead of `for i in x`).
* Evaluate Lazily. For example, use `unwrap_or_else` instead of `unwrap_or` if `default` is not a literal.

## Unsolicited code

* Use return-type overloading a lot. Specifically, code that uses non-trivial `.into` too often. This is because the result of type inference may be counter-intuitive. In this case, it is recommended to use `from` instead.

* Codes that use `Deref` a lot. This causes practically the same problem as inheritance.

## Code that changes its decision depending on the context

* Define unused helper methods.
* Uses `unwrap`, `clone`. In some cases, there is no choice but to do so.
