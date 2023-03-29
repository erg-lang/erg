# pattern matching, refutable

## Patterns available in Erg

### variable pattern

```python
# basic assignments
i = 1
# with type
i: Int = 1
# with anonymous type
i: {1, 2, 3} = 2

# functions
fn x = x + 1
# equals
fn x: Add(Int) = x + 1
# (anonymous) function
fn = x -> x + 1
fn: Int -> Int = x -> x + 1

# higher-order type
a: [Int; 4] = [0, 1, 2, 3]
# or
a: Array Int, 4 = [0, 1, 2, 3]
```

### Literal patterns

```python
# Raise a TypeError if `i` cannot be determined to be 1 at compile time.
# omit `_: {1} = i`
1 = i

# simple pattern matching
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"

# fibonacci function
fib0 = 0
fib1 = 1
fibn: Nat = fibn-1 + fibn-2
```

### constant pattern

```python
cond=False
match! cond:
    True => print! "cond is True"
    _ => print! "cond is False"

PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### Refinement pattern

```python,checker_ignore
# these two are the same
Array(T, N: {N | N >= 3})
Array(T, N | N >= 3)

f M, N | M >= 0, N >= 1 = ...
f(1, 0) # TypeError: N (2nd parameter) must be 1 or more
```

### discard (wildcard) pattern

```python
_ = 1
_: Int = 1
zero_ = 0
right(_, r) = r
```

If not constrained by context, `_` is of type `Obj`.

### Variable length patterns

It is used in combination with the tuple/array/record pattern described later.

```python
[i,...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### Tuple pattern

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# If not nested, () can be omitted (1, 2 are treated as (1, 2))
m, n = 1, 2

f(x, y) = ...
```

### array pattern

```python
[i, j] = [1, 2]
[[k, l], _] = [[1, 2], [3, 4]]

length[] = 0
length[_, ...rest] = 1 + lengthrest
```

#### record pattern

```python
record = {i = 1; j = 2; k = 3}
{j; ...} = record # i, k will be freed

{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age

f {x: Int; y: Int} = ...
```

### Data class pattern

```python
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p

Nil T = Class Impl := Phantom T
Cons T = Inherit {head = T; rest = List T}
List T = Enum Nil(T), Cons(T)
List T.
    first self =
        match self:
            Cons::{head; ...} -> x
            _ -> ...
    second self =
        match self:
            Cons::{rest=Cons::{head; ...}; ...} -> head
            _ -> ...
```

### enumeration pattern

*Actually, it's just an enumeration type

```python
match x:
    i: {1, 2} -> "one or two: \{i}"
    _ -> "other"
```

### range pattern

*Actually, it is just an interval type.

```python
# 0 < i < 1
i: 0<..<1 = 0.5
# 1 < j <= 2
_: {[I, J] | I, J: 1<..2} = [1, 2]
# 1 <= i <= 5
match i
    i: 1..5 -> ...
```

### Things that aren't patterns, things that can't be patterned

A pattern is something that can be uniquely specified. In this respect pattern matching differs from ordinary conditional branching.

Condition specifications are not unique. For example, to check if the number `n` is even, the orthodox is `n % 2 == 0`, but you can also write `(n / 2).round() == n / 2`.
A non-unique form is not trivial whether it works correctly or is equivalent to another condition.

#### set

There is no set pattern. Because the set has no way to uniquely retrieve the elements.
You can retrieve them by iterator, but the order is not guaranteed.

<p align='center'>
    <a href='./27_object_system.md'>Previous</a> | <a href='./29_comprehension.md'>Next</a>
</p>
