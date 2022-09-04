# Subtyping

In Erg, class inclusion can be determined with the comparison operators `<`, `>`.

```erg
Nat < Int
Int < Object
1... _ < Nat
{1, 2} > {1}
{=} > {x = Int}
{I: Int | I >= 1} < {I: Int | I >= 0}
```

Note that this has a different meaning than the `<:` operator. It declares that the class on the left-hand side is a subtype of the type on the right-hand side, and is meaningful only at compile-time.

```erg
C <: T # T: StructuralType
f|D <: E| ...

assert F < G
```

You can also specify `Self <: Add` for a polymorphic subtype specification, for example ``Self(R, O) <: Add(R, O)``.

## Structural types and class type relationships

Structural types are types for structural typing and are considered to be the same object if they have the same structure.

```erg
T = Structural {i = Int}
U = Structural {i = Int}

assert T == U
t: T = {i = 1}
assert t in T
assert t in U
```

In contrast, classes are types for notational typing and cannot be compared structurally to types and instances.

```erg
C = Class {i = Int}
D = Class {i = Int}

assert C == D # TypeError: cannot compare classes
c = C.new {i = 1}
assert c in C
assert not c in D
```

## Subtyping of subroutines

Arguments and return values of subroutines take only a single class.
In other words, you cannot directly specify a structural type or a trait as the type of a function.
It must be specified as "a single class that is a subtype of that type" using the partial type specification.

```erg
# OK
f1 x, y: Int = x + y
# NG
f2 x, y: Add = x + y
# OK
# A is some concrete class
f3<A <: Add> x, y: A = x + y
```

Type inference in subroutines also follows this rule. When a variable in a subroutine has an unspecified type, the compiler first checks to see if it is an instance of one of the classes, and if not, looks for a match in the scope of the trait. If it still cannot find one, a compile error occurs. This error can be resolved by using a structural type, but since inferring an anonymous type may have unintended consequences for the programmer, it is designed to be explicitly specified by the programmer with `Structural`.

## Class upcasting

```erg
i: Int
i as (Int or Str)
i as (1..10)
i as {I: Int | I >= 0}
```
