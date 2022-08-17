# Guidelines for Rust code

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
