# Array! T

A type that represents a variable-length array. Use when the length is not known at compile time. There is a syntactic sugar called `[T]!`.
Defined by `Array! T = ArrayWithMutLength! T, !_`.