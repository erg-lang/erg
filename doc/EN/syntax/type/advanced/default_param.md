# Function type with default arguments

First, let's look at an example of using default arguments.

```python
f: (Int, Int, z := Int) -> Int
f(x, y, z := 0) = x + y + z

g: (Int, Int, z := Int, w := Int) -> Int
g(x, y, z := 0, w := 1) = x + y + z + w

fold: ((Int, Int) -> Int, [Int], acc := Int) -> Int
fold(f, [], acc) = acc
fold(f, arr, acc := 0) = fold(f, arr[1..], f(acc, arr[0]))
assert fold(f, [1, 2, 3]) == 6
assert fold(g, [1, 2, 3]) == 8
```

Arguments after `:=` are default arguments.
The subtyping rules are as follows.

```python
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) <: ((X, ...) -> Z)
```

The first means that a function with default arguments can be identified with a function without.
The second means that any default argument can be omitted.