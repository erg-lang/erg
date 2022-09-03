# Unification of terminology

## Accessibility, Visibility

Use Visibility.

## Complement (negative type, complementary type)

Use negative types. The result of Complement is not necessarily Not type.

## Diff (difference type, exclusion type, direct difference type)

Use exclusion types. The result of Diff is not always Not type.

## Intersection (intersection type, intersection type, Cartesian product type)

Use intersection types. Do not use Cartesian product types. This is because there is also a usage that regards a tuple as a Cartesian product type.
However, from the point of view of attribute subtyping, it is essentially equivalent to Erg's And type.
Also, the result of Intersection is not necessarily And type. For example `{1, 2, 3} and {1, 2} == {1, 2}`.

## Translation of Nominal subtyping

There are nominal/nominal/nominal subtyping, but use nominal subtyping.

## Ratio type translation

Use rational numbers. Since Float is provided separately, it is not called a floating-point number type.

## Union

Use a union type. The result of Union is not necessarily Or type.

## Type bound, Type constraint

A list of predicate expressions given to quantified and sieve types. Use type bounds.

## subroutines, routines, subprograms

Use subroutines.

## Referentially transparent/not, with/without side effects

Use with/without side effects.

## identifiers, algebra, variables, names, symbols

In its original meaning,

* Symbol: Characters (except symbols, control characters, etc.) that are solid-written in source code that are not string objects (not enclosed in ""). Symbols exist as primitive types in Ruby, Lisp, etc., but they are not treated as objects in Erg.
* Identifier: A symbol that (and can) refer to some object, not a reserved word. For example, in Python class and def cannot be used as identifiers. Since Erg has no reserved words, all symbols can be used as identifiers except some symbols.
* Name: Almost same meaning as identifier. It is sometimes used synonymously with algebra in Erg.
* Algebra name: equivalent to identifier in Erg. In C, function names are identifiers, not algebraic names. "Algebra" refers to the language feature itself that allows you to assign objects with `=` (variable assignment operator) or `=` (constant assignment operator).

``` erg
algebraic name <: (name == identifier) ​​<: symbol
variable + constant == algebra
```

However, what should be called "algebra" is often called "variable". This is the effect of mathematical terminology.
A variable whose value content can change is a mutable variable, and a variable whose value content does not change is an immutable variable.
Note that constants are always immutable.

Algebraic names and names are not used in Erg, and uniform identifiers are used.
However, in general, `v` with `v = 1` is called "Variable v", and `C` with `C = 1` is called "Constant C". .

## Attribute, Field, Property

Attribute, use attributes.
By the way, a record is a function that can define an object with element attributes without a class.

## Application, Call

Giving arguments to a subroutine object and getting a result.
Use Call. This is because Application has a usage of "applied software".

## Array, List

Use Arrays. Erg arrays are (generally) contiguous in memory.
List refers to a so-called linked list, or a list as a Python data type.

## procedures, procedures

Standardize on procedures. Subroutine is a generic term for functions (and operators), procedures and methods. Callable is also anything that implements `__call__`.

## lambda functions, lambda expressions, anonymous functions, anonymous functions

Unify with anonymous functions. In English, Lambda can be used to shorten the number of characters, but the official name is Anonymous function.
Also, Erg's anonymous functions are not anonymous, so we don't use them.