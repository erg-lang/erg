# Comprehension

Array with `[expr | (name <- iterable)+ (predicate)*]`,
set with `{expr | (name <- iterable)+ (predicate)*}`,
You can create a Dict with `{key: value | (name <- iterable)+ (predicate)*}`.

The first part of the clauses separated by `|` is called the layout clause (location clause), the second part is called the bind clause (binding clause), and the third part is called the guard clause (conditional clause).
A guard clause can be omitted, but a bind clause cannot be omitted, and a guard clause cannot precede a bind clause.

Comprehension example

```python
# the layout clause is i
# bind clause is i <- [0, 1, 2]
assert [i | i <- [0, 1, 2]] == [0, 1, 2]

# layout clause is i / 2
# bind clause is i <- 0..2
assert [i/2 | i <- 0..2] == [0.0, 0.5, 1.0]

# layout clause is (i, j)
# bind clause i <- 0..2, j <- 0..2
# guard clause is (i + j) % 2 == 0
assert [(i, j) | i <- 0..2; j <- 0..2; (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]

assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

Erg comprehensions are inspired by Haskell, but with some differences.
For Haskell list comprehensions, the order of variables makes a difference in the result, but in Erg it doesn't matter.

``` haskell
-- Haskell
[(i, j) | i <- [1..3], j <- [3..5]] == [(1,3),(1,4),(1,5),(2 ,3),(2,4),(2,5),(3,3),(3,4),(3,5)]
[(i, j) | j <- [3..5], i <- [1..3]] == [(1,3),(2,3),(3,3),(1 ,4),(2,4),(3,4),(1,5),(2,5),(3,5)]
```

```python
# Erg
assert [(i, j) | i <- 1..<3; j <- 3..<5] == [(i, j) | j <- 3..<5; i <- 1.. <3]
```

This specification is the same as that of Python.

```python
# Python
assert [(i, j) for i in range(1, 3) for j in range(3, 5)] == [(i, j) for j in range(3, 5) for i in range(1, 3)]
```

## Sieve type

Similar to comprehensions are sieve types. A sieve type is a type (enumerated type) created in the form `{Name: Type | Predicate}`.
In the case of the sieve type, only one Name can be specified and the layout cannot be specified (however, multiple values ​​can be handled if it is a tuple type), and the Predicate can be calculated at compile time, that is, only a constant expression can be specified.

```python
Nat = {I: Int | I >= 0}
# If the predicate expression is only and, it can be replaced with ;
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./26_pattern_matching.md'>上一页</a> | <a href='./28_spread_syntax.md'>下一页</a>
</p>