# Type Bound

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/19_bound.md%26commit_hash%3D417bfcea08ed0e09f715f5d272842510fca8f6dd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/19_bound.md&commit_hash=417bfcea08ed0e09f715f5d272842510fca8f6dd)

A type boundary is a condition on the type specification. The guard (guard clause) is the function that makes this possible.
In addition to function signatures and anonymous function signatures, sieve types can also use this feature.
The guard is described after the return type.

## Predicate Expressions (Predicate)

The condition that a variable satisfies can be specified by an expression (predicate) that returns a `Bool`.
You can use [value object](./08_value.md) and operators. Compile-time functions may be supported in future versions.

```erg
f a: [T; N] | T, N, N > 5 = ...
g a: [T; N | N > 5] | T, N = ...
Odd = {I: Int | I % 2 == 1}
R2Plus = {(L, R) | L, R: Ratio; L > 0 and R > 0}
GeneralizedOdd = {I | U; I <: Div(Nat, U); I % 2 == 0}
```
