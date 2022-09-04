# Guidelines for Rust code

## local rules

* Use `log!` for output for debugging (use `println!` etc. for output processing that is also necessary for release).
* Unused or internal variables/methods (private and used only for specific functions) must be prefixed with `_`. If you want to avoid conflicts with reserved words, add one `_` to the end.

## Recommended code

* Define and use domain-specific Enums instead of numeric enumerations or bools.
* Keep access modifiers to a minimum. Prioritize using `pub(mod)` or `pub(crate)` even when publishing.
* Convert an iterable object in a for expression explicitly to an iterator (`for i in x.iter()` instead of `for i in x`).
* Lazy evaluation. For example, if `default` is non-literal, use `unwrap_or_else` instead of `unwrap_or`.

## Code not encouraged

* Make heavy use of return type overloading. Specifically code that uses a lot of non-obvious `.into`. This is because type inference results can be counter-intuitive. In this case it is recommended to use `from` instead.
* Make heavy use of `Deref`. This effectively poses the same problem as inheritance.

## Code that makes decisions based on context

* Define unused helper methods.
* Use `unwrap` and `clone` a lot. In some cases there is nothing better than doing so.