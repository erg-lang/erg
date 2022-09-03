# Quick Tour

The documentation below `syntax` is written with the aim of being understandable even for programming beginners.
For those who have already mastered languages ​​such as Python, Rust, Haskell, etc., it may be a bit verbose.

So, here's an overview of the Erg grammar.
Please think that the parts not mentioned are the same as Python.

## variables, constants

Variables are defined with `=`. As with Haskell, variables once defined cannot be changed. However, it can be shadowed in another scope.

``` erg
i = 0
if True:
    i = 1
assert i == 0
```

Anything starting with an uppercase letter is a constant. Only things that can be computed at compile time can be constants.
Also, a constant is identical in all scopes since its definition.

``` erg
PI = 3.141592653589793
match random.random!(0..10):
    PIs:
        log "You get PI, it's a miracle!"
```

## declaration

Unlike Python, only the variable type can be declared first.
Of course, the declared type and the type of the object actually assigned to must be compatible.

``` erg
i: Int
i = 10
```

## Functions

You can define it just like in Haskell.

``` erg
fib0 = 0
fib1 = 1
fibn = fib(n - 1) + fib(n - 2)
```

An anonymous function can be defined like this:

``` erg
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## operator

The Erg-specific operators are:

### mutating operator (!)

It's like `ref` in Ocaml.

``` erg
i = !0
i.update! x -> x + 1
assert i == 1
```

## procedures

Subroutines with side effects are called procedures and are marked with `!`.

``` erg
print! 1 # 1
```

## generic function (polycorrelation)

``` erg
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## Records

You can use the equivalent of records in ML-like languages ​​(or object literals in JS).

``` erg
p = {x = 1; y = 2}
```

## Ownership

Ergs are owned by mutable objects (objects mutated with the `!` operator) and cannot be rewritten from multiple places.

``` erg
i = !0
j = i
assert j == 0
i#MoveError
```

Immutable objects, on the other hand, can be referenced from multiple places.

## Visibility

Prefixing a variable with `.` makes it a public variable and allows it to be referenced from external modules.

``` erg
# foo.er
.x = 1
y = 1
```

``` erg
foo = import "foo"
assert foo.x == 1
foo.y # VisibilityError
```

## pattern matching

### variable pattern

``` erg
# basic assignments
i = 1
# with type
i: Int = 1
# functions
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### Literal patterns

``` erg
# if `i` cannot be determined to be 1 at compile time, TypeError occurs.
# shorthand of `_: {1} = i`
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

``` erg
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### discard (wildcard) pattern

``` erg
_ = 1
_: Int = 1
right(_, r) = r
```

### Variable length patterns

Used in combination with the tuple/array/record pattern described later.

``` erg
[i,...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### Tuple pattern

``` erg
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# If not nested, () can be omitted (1, 2 are treated as (1, 2))
m, n = 1, 2
```

### array pattern

``` erg
length[] = 0
length[_, ...rest] = 1 + lengthrest
```

#### record pattern

``` erg
{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### Data class pattern

``` erg
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## Comprehensions

``` erg
odds = [i | i <- 1..100; i % 2 == 0]
```

## class

Erg does not support multiple/multilevel inheritance.

## Traits

They are similar to Rust traits, but in a more literal sense, allowing composition and decoupling, and treating attributes and methods as equals.
Also, it does not involve implementation.

``` erg
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## patch

You can give implementations to classes and traits.

## Sieve type

A predicate expression can be type-restricted.

``` erg
Nat = {I: Int | I >= 0}
```

## parametric type with value (dependent type)

``` erg
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```