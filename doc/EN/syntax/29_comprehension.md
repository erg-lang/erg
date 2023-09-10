# Comprehension

You can create an Array with `[(expr |)? (name <- iterable;)+ (| predicate)?]`,
a set with `{(expr |)? (name <- iterable;)+ (| predicate)?}`,
a Dict with `{(key: value |)? (name <- iterable;)+ (| predicate)?}`.

The first part of the clauses separated by `|` is called the layout clause, the second part is called the binding clause, and the third part is called the guard clause.
Either a guard clause or a layout clause can be omitted, but bind clauses cannot be omitted, and a guard clause cannot precede a bind clause.

Comprehension example

```python
# layout clause: i
# bind clause: i <- [0, 1, 2]
assert [i | i <- [0, 1, 2]] == [0, 1, 2]

# If you only want to filter, you can omit the layout clause
# This is same as [0, 1, 2].iter().filter(i -> i % 2 == 0).into_array()
assert [i <- [0, 1, 2] | i % 2 == 0] == [0, 2]

# layout clause: i / 2
# bind clause: i <- 0..2
assert [i/2 | i <- 0..2] == [0.0, 0.5, 1.0]

# layout clause: (i, j)
# bind clause: i <- 0..2, j <- 0..2
# guard clause: (i + j) % 2 == 0
assert [(i, j) | i <- 0..2; j <- 0..2 | (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]

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

## Refinement type

Similar to comprehensions are refinement types. A refinement type is a type (enumerated type) created in the form `{Name: Type | Predicate}`.
In the case of the refinement type, only one Name can be specified and the layout cannot be specified (however, multiple values ​​can be handled if it is a tuple type), and the Predicate can be calculated at compile time, that is, only a constant expression can be specified.

```python
Nat = {I: Int | I >= 0}
# If the predicate expression is only and, it can be replaced with ;
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./28_pattern_matching.md'>Previous</a> | <a href='./30_spread_syntax.md'>Next</a>
</p>
