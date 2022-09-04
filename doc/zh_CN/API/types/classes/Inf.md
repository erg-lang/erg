# Inf

Inf is a class whose only instance is inf.
The main use of inf is with interval types.
For example, integer types greater than or equal to 2 are `2..<inf`, and real numbers less than or equal to 0 are `-inf<..0.0`.
Since inf is not a number in the usual sense, the four arithmetic operations cannot be defined as it is,
So-called extended number classes such as ExtNat are provided in the library.