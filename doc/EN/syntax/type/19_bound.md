# Type Bound

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
