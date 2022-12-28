
# Compound Type

## Tuple Type

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

Tuples have a partial type rule for length as well as type inside.
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

Arrays like the one below are not valid types. It is an intentional design to emphasize that the elements of the array are equalized.

```erg
[Int, Str]
```

Because of this, detailed information about each element is lost. To preserve this, a Refinement type is used.

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## Set Type

```erg
{}, {X}, ...
```

The Set type itself has no length information. This is because duplicate elements are eliminated in sets, but duplicate elements cannot generally be determined at compile time. In the first place, the length of the information is not very meaningful in a Set.

`{}` is the empty set, a subtype of all types.

## Dict Type

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

## Record Type

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

There is no subtype relationship between private and public type of attribute, however they can be converted to each other by `.Into`.

```erg
r = {i = 1}.Into {.i = Int}
assert r.i == 1
```

## Function Type

```erg
() -> ()
Int -> Int
(Int, Str) -> Bool
(x: Int, y: Int) -> Int
(x := Int, y := Int) -> Int
(...objs: Obj) -> Str
(Int, Ref Str!) -> Int
|T: Type|(x: T) -> T
|T: Type|(x: T := NoneType) -> T # |T: Type|(x: T := X, y: T := Y) -> T (X != Y) is invalid
```
