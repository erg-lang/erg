# Array! T, N

A type that represents a variable-length array. Use when the length is not known at compile time. There is a syntactic sugar called `[T; N]!`.
`N` can be emitted (`[T; _]!`) and if then, the length is not fixed.
