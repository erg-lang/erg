# Type Bound

Type bounds add conditions to type specifications. A function that realizes this is a guard (guard clause).
This feature is available for function signatures, anonymous function signatures, as well as refinement types.
Guards are written after the return type.

## Predicate

You can specify the condition that the variable satisfies with an expression (predicate expression) that returns `Bool`.
Only [value objects](./08_value.md) and operators can be used. Compile-time functions may be supported in future versions.

```python
f a: [T; N] | T, N, N > 5 = ...
g a: [T; N | N > 5] | T, N = ...
Odd = {I: Int | I % 2 == 1}
R2Plus = {(L, R) | L, R: Ratio; L > 0 and R > 0}
GeneralizedOdd = {I | U; I <: Div(Nat, U); I % 2 == 0}
```

<p align='center'>
    <a href='./18_mut.md'>Previous</a> | Next
</p>
