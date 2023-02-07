# Quick Tour

The documentation below `syntax` is written with the aim of being understandable even for programming beginners.
For those who have already mastered languages ​​such as Python, Rust, Haskell, etc., it may be a bit verbose.

So, here's an overview of the Erg grammar.
Please think that the parts not mentioned are the same as Python.

## Basic calculation

Erg has a strict type. However, types are automatically casting if subtypes due to the flexibility provided by classes and traits (see [API](../API) for details).

In addition, different types can be calculated for each other as long as the type is a numeric type.

```python
a = 1 # 1: Nat
b = a - 10 # -9: Int
c = b / 2 # -4.5: Float
d = c * 0 # -0.0: Float
e = f // 2 # 0: Nat
```

If you do not want to allow unexpected type widening, you can specify the type at declaration time to detect them as errors at compile time.

```python
a = 1
b: Int = a / 2
# error message
Error[#0047]: File <stdin>, line 1, in <module>
2│ b: Int = a / 2
   ^
TypeError: the type of b is mismatched:
expected:  Int
but found: Float
```

## Boolean type

`True` and `False` are singletons of the Boolean type, but they can also be cast to the Int type.

Therefore, they can be compared if they are of type Int, but comparisons with other types will result in an error.

```python
True == 1 # OK
False == 0 # OK
True == 1.0 # NG
False == 0.0 # NG
True == "a" # NG
```

## Variables, constants

Variables are defined with `=`. As with Haskell, variables once defined cannot be changed. However, it can be shadowed in another scope.

```python
i = 0
if True:
    i = 1
assert i == 0
```

Anything starting with an uppercase letter is a constant. Only things that can be computed at compile time can be constants.
Also, a constant is identical in all scopes since its definition.
This property allows constants to be used in pattern matching.

```python
PI = 3.141592653589793
match random.random!(0..10):
    PI ->
        log "You get PI, it's a miracle!"
```

## declaration

Unlike Python, only the variable type can be declared first.
Of course, the declared type and the type of the object actually assigned to must be compatible.

```python
i: Int
i = 10
```

## Functions

You can define it just like in Haskell.

```python
fib 0 = 0
fib 1 = 1
fib n = fib(n - 1) + fib(n - 2)
```

An anonymous function can be defined like this:

```python
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## operator

The Erg-specific operators are:

### mutating operator (!)

It's like `ref` in Ocaml.

```python
i = !0
i.update! x -> x + 1
assert i == 1
```

## Procedures

Subroutines with side effects are called procedures and are marked with `!`.
Functions are subroutines that do not have side effects (pure).

You cannot call procedures in functions.
This explicitly isolates side effects.

```python
print! 1 # 1
```

## generic function (polycorrelation)

```python
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## Records

You can use the equivalent of records in ML-like languages ​​(or object literals in JS).

```python
p = {x = 1; y = 2}
assert p.x == 1
```

## Ownership

Ergs are owned by mutable objects (objects mutated with the `!` operator) and cannot be rewritten from multiple places.

```python
i = !0
j = i
assert j == 0
i# MoveError
```

Immutable objects, on the other hand, can be referenced from multiple places.

## Visibility

Prefixing a variable with `.` makes it a public variable and allows it to be referenced from external modules.

```python
# foo.er
.x = 1
y = 1
```

```python
foo = import "foo"
assert foo.x == 1
foo.y # VisibilityError
```

## Pattern matching

### Variable pattern

```python
# basic assignments
i = 1
# with type
i: Int = 1
# functions
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### Literal patterns

```python
# if `i` cannot be determined to be 1 at compile time, TypeError occurs.
# shorthand of `_: {1} = i`
1 = i
# simple pattern matching
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# fibonacci function
fib 0 = 0
fib 1 = 1
fib n: Nat = fibn-1 + fibn-2
```

### Constant pattern

```python
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### Discard (wildcard) pattern

```python
_ = 1
_: Int = 1
right(_, r) = r
```

### Variable length patterns

Used in combination with the tuple/array/record pattern described later.

```python
[i, *j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, *rest: T) = fst
assert first(1, 2, 3) == 1
```

### Tuple pattern

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# If not nested, () can be omitted (1, 2 are treated as (1, 2))
m, n = 1, 2
```

### Array pattern

```python
length [] = 0
length [_, *rest] = 1 + length rest
```

#### Record pattern

```python
{sin; cos; tan} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### Data class pattern

```python
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## Comprehensions

```python
odds = [i | i <- 1..100; i % 2 == 0]
```

## Class

Erg does not support multiple inheritance.

Classes are non-inheritable by default, and you must define inheritable classes with the `Inheritable` decorator.

```python
@Inheritable
Point2D = Class {x = Int; y = Int}

Point3D = Inherit Point2D, Base := {x = Int; y = Int; z = Int}
```

## Trait

They are similar to Rust traits, but in a more literal sense, allowing composition and decoupling, and treating attributes and methods as equals.
Also, it does not involve implementation.

```python
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## Patch

You can retrofit a class or trait with an implementation.

````python
Invert = Patch Bool
Invert.
    invert self = not self

assert False.invert()
````

## Refinement types

A predicate expression can be type-restricted.

```python
Nat = {I: Int | I >= 0}
```

## parametric type with value (dependent type)

```python
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```
