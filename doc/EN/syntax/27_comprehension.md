# Comprehension

An array can be created by `[expr | (name <- iterable)+ (predicate)*]`,
And a set can be created by `{expr | (name <- iterable)+ (predicate)*}`.

Dict can be created by `{key: value | (name <- iterable)+ (predicate)*}`.

The first part of a clause delimited by `|` is called a layout clause, the second part is called a bind clause, and the third part is called a guard clause.
The guard clause can be omitted, but not the bind clause, and the guard clause cannot be placed before the bind clause.

e.g.

```erg
assert [i | i <- [0, 1, 2]] == [0, 1, 2]]
assert [i / 2 | i <- 0..2] == [0.0, 0.5, 1.0]]
assert [(i, j) | i <- 0..2; j <- 0..2; (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]
assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

Erg's comprehension notation is influenced by Haskell, but there are some differences.
In Haskell's list comprehensions, the order of variables makes a difference in the result, but not in Erg.

```haskell
-- Haskell
[(i, j) | i <- [1..3], j <- [3..5]] == [(1,3),(1,4),(1,5),(2,3),(2,4),(2,5),(3,3),(3,4),(3,5)]
[(i, j) | j <- [3..5], i <- [1..3]] == [(1,3),(2,3),(3,3),(1,4),(2,4),(3,4),(1,5),(2,5),(3,5)]
```

```erg
# Erg
assert [(i, j) | i <- 1. <3; j <- 3.. <5] == [(i, j) | j <- 3.. <5; i <- 1.. <3]
```

これはPythonと同じである。

```python
# Python
assert [(i, j) for i in range(1, 3) for j in range(3, 5)] == [(i, j) for j in range(3, 5) for i in range(1, 3)]
```

## Refinement type

Similar to comprehensions are refinement types. A refinement type is a type (enumerated type) in the form `{Name: Type | Predicate}`.
In the case of a refinement type, Name is limited to one and the layout cannot be specified (but multiple values can be handled by using a tuple type, for example), and Predicate must be a compile-time computation, i.e., a constant expression.

```erg
Nat = {I: Int | I >= 0}
# If the predicate expression is and only, it can be replaced by ;.
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='. /26_pattern_matching.md'>Previous</a> | <a href='. /28_spread_syntax.md'>Next</a>
</p>
