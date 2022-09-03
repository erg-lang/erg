# Kind

Everything is typed in Erg. Types themselves are no exception. __kind__ represents the “type of type”. For example, `Int` belongs to `Type`, just as `1` belongs to `Int`. `Type` is the simplest kind, the __atomic kind__. In type-theoretic notation, `Type` corresponds to `*`.

In the concept of kind, what is practically important is one or more kinds (multinomial kind). One-term kind, for example `Option`, belongs to it. A unary kind is represented as `Type -> Type` [<sup id="f1">1</sup>](#1). A __container__ such as `Array` or `Option` is specifically a polynomial kind that takes a type as an argument.
As the notation `Type -> Type` indicates, `Option` is actually a function that receives a type `T` and returns a type `Option T`. However, since this function is not a function in the usual sense, it is usually called the unary kind.

Note that `->` itself, which is an anonymous function operator, can also be seen as a kind when it receives a type and returns a type.

Also note that a kind that is not an atomic kind is not a type. Just as `-1` is a number but `-` is not, `Option Int` is a type but `Option` is not. `Option` etc. are sometimes called type constructors.

``` erg
assert not Option in Type
assert Option in Type -> Type
```

So code like the following will result in an error:
In Erg, methods can only be defined in atomic kinds, and the name `self` cannot be used anywhere other than the first argument of a method.

``` erg
#K is an unary kind
K: Type -> Type
K T = Class...
K.
    foo x = ... # OK, this is like a so-called static method
    bar self, x = ... # TypeError: cannot define a method to a non-type object
K(T).
    baz self, x = ... # OK
```

Examples of binary or higher kinds are `{T: U}`(: `(Type, Type) -> Type`), `(T, U, V)`(: `(Type, Type, Type) - > Type`), ... and so on.

There is also a zero-term kind `() -> Type`. This is sometimes equated with an atomic kind in type theory, but is distinguished in Erg. An example is `Class`.

``` erg
Nil = Class()
```

## Containment of kind

There is also a partial type relation, or rather a partial kind relation, between multinomial kinds.

``` erg
K T = ...
L = Inherit K
L<: K
```

That is, for any `T`, if `L T <: K T`, then `L <: K`, and vice versa.

``` erg
∀T. L T <: K T <=> L <: K
```

## higher kind

There is also a higher-order kind. This is a kind of the same concept as a higher-order function, a kind that receives a kind itself. `(Type -> Type) -> Type` is a higher kind. Let's define an object that belongs to a higher kind.

``` erg
IntContainerOf K: Type -> Type = K Int
assert IntContainerOf Option == Option Int
assert IntContainerOf Result == Result Int
assert IntContainerOf in (Type -> Type) -> Type
```

The bound variables of a polynomial kind are usually denoted as K, L, ..., where K is K for Kind.

## set kind

In type theory, there is the concept of a record. This is almost the same as the Erg record [<sup id="f2">2</sup>](#2).

``` erg
# This is a record, and it corresponds to what is called a record in type theory
{x = 1; y = 2}
```

When all record values ​​were types, it was a kind of type called a record type.

``` erg
assert {x = 1; y = 2} in {x = Int; y = Int}
```

A record type types a record. A good guesser might have thought that there should be a "record kind" to type the record type. Actually it exists.

``` erg
log Typeof {x = Int; y = Int} # {{x = Int; y = Int}}
```

A type like `{{x = Int; y = Int}}` is a record kind. This is not a special notation. It is simply an enumeration type that has only `{x = Int; y = Int}` as an element.

``` erg
Point = {x = Int; y = Int}
Pointy = {Point}
```

An important property of record kind is that if `T: |T|` and `U <: T` then `U: |T|`.
This is also evident from the fact that enums are actually syntactic sugar for sieve types.

``` erg
# {c} == {X: T | X == c} for normal objects, but
# Equality may not be defined for types, so |T| == {X | X <: T}
{Point} == {P | P <: Point}
```

`U <: T` in type constraints is actually syntactic sugar for `U: |T|`.
A kind that is a set of such types is commonly called a set kind. Setkind also appears in the Iterator pattern.

``` erg
Iterable T = Trait {
    .Iterator = {Iterator}
    .iter = Self(T).() -> Self.Iterator T
}
```

## Type inference for polynomial kinds

``` erg
Container K: Type -> Type, T: Type = Patch K(T, T)
Container (K).
    f self = ...
Option T: Type = Patch T or NoneType
Option(T).
    f self = ...
Fn T: Type = Patch T -> T
Fn(T).
    f self = ...
Fn2 T, U: Type = Patch T -> U
Fn2(T, U).
    f self = ...

(Int -> Int).f() # which one is selected?
```

In the example above, which patch would the method `f` choose?
Naively, `Fn T` seems to be chosen, but `Fn2 T, U` is also possible, `Option T` includes `T` as it is, so any type is applicable, `Container K , T` also matches ``` `->`(Int, Int)```, i.e. ```Container(`->`, Int)``` as ```Int -> Int`. So all four patches above are possible options.

In this case, patches are selected according to the following priority criteria.

* Any `K(T)` (e.g. `T or NoneType`) preferentially matches `Type -> Type` over `Type`.
* Any `K(T, U)` (e.g. `T -> U`) matches `(Type, Type) -> Type` preferentially over `Type`.
*Similar criteria apply for kind of 3 or more.
* The one that requires fewer type variables to replace is chosen. For example, `Int -> Int` is `T -> T` rather than `K(T, T)` (replacement type variables: K, T) or `T -> U` (replacement type variables: T, U). `(replacement type variable: T) is matched preferentially.
* If the number of replacements is also the same, an error is given as being unselectable.

---

<span id="1" style="font-size:x-small"><sup>1</sup> In type theory notation, `*=>*` [↩](#f1)</span>

<span id="2" style="font-size:x-small"><sup>2</sup> There are subtle differences such as visibility. [↩](#f2)</span>