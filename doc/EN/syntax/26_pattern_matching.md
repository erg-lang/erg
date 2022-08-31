# Pattern matching, Irrefutability

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/26_pattern_matching.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/26_pattern_matching.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

## Patterns Available in Erg

### Variable Pattern

```erg
# basic assignment
i = 1
# with type
i: Int = 1
# with anonymous type
i: {1, 2, 3} = 2
# function
fn x = x + 1
# equals
fn x: Add(Int) = x + 1
# (anonymous) function
fn = x -> x + 1
fn: Int -> Int = x -> x + 1
# higher-order type
a: [Int; 4] = [0, 1, 2, 3]
# or
a: Array Int, 4 = [0, 1, 2, 3] # or
```

### Literal Pattern

```erg
# if `i` cannot be determined to be 1 at compile time, TypeError occurs.
# short hand of `_: {1} = i`
1 = i
# simple pattern matching
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# fibonacci function
fib 0 = 0
fib 1 = 1
fib n: Nat = fib n-1 + fib n-2
```

### Constant Pattern

```erg
cond = False
match! cond:
    True => print!
    _ => print! "cond is False"

PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### Refinement Pattern

```erg
Array(T, N: {N | N >= 3})
# == ==
Array(T, N | N >= 3)

f M, N | M >= 0, N >= 1 = ...
f(1, 0) # TypeError: N (2nd parameter) must be 1 or more
```

### Discard (Wildcard) Pattern

```erg
_ = 1
_: Int = 1
zero _ = 0
right(_, r) = r
```

### Varargs Patterns

Used in combination with the tuple/array/record pattern described below.

```erg
[i, . .j] = [1, 2, 3, 4]
assert j == [2, 3, 4].
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### Tuple Pattern

```erg
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# () can be omitted if not nested (1, 2 are treated as (1, 2))
m, n = 1, 2

f(x, y) = ...
```

### Array Pattern

```erg
[i, j] = [1, 2]
[[k, l], _] = [[1, 2], [3, 4]]

length [] = 0
length [_, . .rest] = 1 + length rest
```

### Record Pattern

```erg
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

### Data Class Pattern

```erg
Point = Inherit {x = Int; y = Int}
p = Point.{x = 1; y = 2}
Point.{x; y} = p

Nil T = Class Impl: Phantom T
Cons T = Inherit {head = T; rest = List T}
List T = Enum Nil(T), Cons(T)
List T.
    first self =
        match self:
            Cons.{head; ...} -> x
            _ -> ...
    second self =
        match self:
            Cons.{rest=Cons.{head; ...} ; ...} -> head
            _ -> ...
```

### Enumeration Pattern

* actually just an enumerated type

```erg
match x:
    i: {1, 2} -> "one or two: {i}"
    _ -> "other"
```

### Range Pattern

* actually just an interval type

```erg
# 0 < i < 1
i: 0<... <1 = 0.5
# 1 < j <= 2
_: {[I, J] | I, J: 1<. .2} = [1, 2]
# 1 <= i <= 5
match i
    i: 1..5 -> ...
```

### Non-patterns and Non-patternable Items

A pattern is something that can be uniquely specified. In this respect, pattern matching differs from ordinary conditional branching.

The specification of a condition is not unique. For example, to determine whether the number `n` is even, the orthodox way is `n % 2 == 0`, but it can also be written as `(n / 2).round() == n / 2`.
The non-unique form is non-trivial, whether it works correctly or is equivalent to another condition.

#### Set

There is no pattern for sets. There is no pattern for sets because there is no way to retrieve elements uniquely.
They can be retrieved with an iterator, but the order is not guaranteed.

<p align='center'>
    <a href='./25_object_system.md'>Previous</a> | <a href='./27_comprehension.md'>Next</a>
</p>
