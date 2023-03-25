
# Compound Type

## Tuple Type

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

Tuples have a subtype rule for length as well as type inside.
For any Tuple `T`, `U`, the following holds.

```erg
* T <: () (unit rule)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U.N == T.N => U <: T (oblivion rule)
```

For example, `(Int, Str, Bool) <: (Int, Str)`.
However, these rules do not apply to the tuple-like part of a Function type, because this part is not really the tuples.

```erg
(Int, Int) -> Int !<: (Int,) -> Int
```

In addition, return values of Unit types can be ignored, but return values of other tuple types cannot be ignored.

## Array Type

```erg
[], [X; 0], [X; 1], [X; 2], ..., [X; _] == [X]
```

The same subtype rules exist for arrays as for tuples.

```erg
* T <: [] (unit rule)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U[N] == T[N] => U <: T (oblivion rule)
```

Arrays like the one below are not valid types. It is an intentional design to emphasize that the elements of the array are homogenized.

```erg
[Int, Str]
```

Because of this, detailed information about each element is lost. To preserve this, refinement types can be used.

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## Set Type

```erg
{}, {X; _}, ...
```

Set types have length information, but mostly useless. This is because duplicate elements are eliminated in sets, but duplicate elements cannot generally be determined at compile time.
In the first place, the length of the information is not very meaningful in a Set.

`{}` is the empty set, a subtype of all types. Note that `{X}` is not a set type, but a type that contains only one constant `X`.

## Dict Type

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

All dict types are subtypes of `Dict K, V`. `{X: Y} <: Dict X, Y` and `{X: Y, Z: W} <: Dict X or Z, Y or W`.

## Record Type

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

A private record type is a super type of a public record type.

e.g. `{.i = Int} <: {i = Int}`

## Function Type

```erg
() -> ()
Int -> Int
(Int, Str) -> Bool
# named parameter
(x: Int, y: Int) -> Int
# default parameter
(x := Int, y := Int) -> Int
# variable-length parameter
(*objs: Obj) -> Str
(Int, Ref Str!) -> Int
# qualified parameter
|T: Type|(x: T) -> T
# qualified parameter with default type
|T: Type|(x: T := NoneType) -> T # |T: Type|(x: T := X, y: T := Y) -> T (X != Y) is invalid
```

## Bound Method Type

```erg
Int.() -> Int
Int.(other: Int) -> Int
# e.g. 1.__add__: Int.(Int) -> Int
```

The type `C.(T) -> U` is a subtype of `T -> U`. They are almost the same, but ``C.(T) -> U`` is the type of a method whose receiver type is `C`, and the receiver is accessible via an attribute `__self__`.

<p align='center'>
    <a href='./19_bound.md'>Previous</a> | Next
</p>
