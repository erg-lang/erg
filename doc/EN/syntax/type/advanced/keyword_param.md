# Function type with keyword arguments

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/keyword_param.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/keyword_param.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

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
