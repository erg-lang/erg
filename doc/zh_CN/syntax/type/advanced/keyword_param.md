# Function type with keyword arguments

``` erg
h(f) = f(y: 1, x: 2)
h: |T: Type|((y: Int, x: Int) -> T) -> T
```

The subtyping rules for functions with keyword arguments are as follows.

``` erg
((x: T, y: U) -> V) <: ((T, U) -> V) # x, y are arbitrary keyword parameters
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

This means that keyword arguments can be deleted or replaced.
But you can't do both at the same time.
That is, you cannot cast `(x: T, y: U) -> V` to `(U, T) -> V`.
Note that keyword arguments are attached only to top-level tuples, and not to arrays or nested tuples.

``` erg
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```