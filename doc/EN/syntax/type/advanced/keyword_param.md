# Function type with keyword arguments

```erg
h(f) = f(y: 1, x: 2)
h: |T: Type|((y: Int, x: Int) -> T) -> T
```

The subtyping rules for functions with keyword arguments are as follows.

```erg
((x: T, y: U) -> V) <: ((T, U) -> V) # x, y are arbitrary keyword parameters
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

This means that keyword arguments can be eliminated or replaced.
However, it is not possible to do both at the same time.
That is, `(x: T, y: U) -> V` cannot be cast to `(U, T) -> V`.
Note that keyword arguments are only available in top-level tuples, not in arrays or nested tuples.

```erg
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```
