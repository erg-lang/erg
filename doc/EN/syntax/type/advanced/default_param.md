# Function type with default parameter

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/default_param.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/default_param.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

First, let's look at an example of the use of default parameters.

```erg
f: (Int, Int, z := Int) -> Int
f(x, y, z := 0) = x + y + z

g: (Int, Int, z := Int, w := Int) -> Int
g(x, y, z := 0, w := 1) = x + y + z + w

fold: ((Int, Int) -> Int, [Int], acc := Int) -> Int
fold(f, [], acc) = acc
fold(f, arr, acc := 0) = fold(f, arr[1...]) , f(acc, arr[0]))
assert fold(f, [1, 2, 3]) == 6
assert fold(g, [1, 2, 3]) == 8
```

The parameters after `:=` are default parameters.
The subtyping rules are as follows.

```erg
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) -> Z) <: ((X, ...) -> Z)
```

The first means that a function with a default parameter is identical to a function without it.
The second means that any default parameter can be omitted.
