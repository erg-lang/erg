# Erg design's "Why" and Answers

## Why do Erg's have an ownership system but also coexist with the reference counting system?

This is because Erg's motivation for introducing an ownership system is not for "memory management that does not rely on GC" like Rust.
To begin with, Erg is currently a language that is transpiled into Python bytecode, so GC is used after all.
Erg's aim in introducing the ownership system is "localization of mutable state"; in Erg, mutable objects have the concept of ownership.
This is because shared mutable state can easily become a breeding ground for bugs and even violates type safety (see [here](./syntax/type/advanced/shared.md#SharedReference).

## Why are the parentheses surrounding type parameters || instead of <> or []?

Because `<>` and `[]` cause syntax conflicts.

```erg
id[T: Type] [t]: [T] = t
y = id[Int] # is this a function or array accessing?

id<T: Type> {t: T} = t
y = (id < Int, 1 > 1) # is this a tuple of bools or a call?

id{T: Type} {t: T} = t # conflicts with record pattern
y = id{Int}

id|T: Type| t: T = t
y = id|Int| # OK
```

## The type of {i = 1} is {i = Int}, but in OCaml and other languages it is {i: Int}. Why did Erg adopt the former syntax?

Because Erg is designed to treat the type itself as a value.

```erg
A = [Int; 3]
assert A[2] == Int
T = (Int, Str)
assert T.1 == Str
D = {Int: Str}
assert D[Int] == Str
S = {.i = Int}
assert S.i == Int
```

## Are there any plans to implement macros in Erg?

No. Macros have four main purposes: first, they are compile-time calculations. This is the role of compile-time functions in Erg.
The second is to delay code execution. The third is common processing, for which polymorphic functions and all-symmetric types are better solutions than macros.
Thus, the type system in Erg takes most of the functionality of macros on its shoulders, so there is no motivation to implement it.

## Why is there no exception mechanism in Erg?

Because in many cases, error handling with the `Result` type is a better solution. The `Result` type is a common error handling technique used in many relatively new programming languages.

Erg allows the `?` operator allows you to write without much awareness of errors.

```erg
read_file!() =
    f = open!("foo.txt")? # If it fails, it returns an error immediately, so `f` is of type `File`
    f.read_all!()

# `try` procedure can be used to catch and process like an exception
try!:
    do!:
        s = read_file!()?
        print! s
    e =>
        # Blocks to execute when an error occurs
        print! e
        exit 1
```

When Python functions are imported, by default they are all considered to be functions with exceptions (if they are not typed manually), and their return type is of type `Result`.
If it is known that an exception will not be sent, it is made explicit by `assert`.

## Erg seems to eliminate Python features that are considered bad practice, why didn't you do away with inheritance?

Because some classes in the Python library are designed to be inherited, and completely eliminating inheritance would cause problems in their use.
However, in Erg, classes are `Final` by default, and multiple and multi-layer inheritance is prohibited in principle, so inheritance can be used relatively safely.

## Why does subtype inference for polymorphic functions point to nominal traits by default?

Because pointing to structural traits by default complicates type specification and may introduce unintended behavior by the programmer.

```erg
# If T is a subtype of a structural trait...
# f: |T <: Structural Trait {.`_+_` = Self.(Self) -> Self; .`_-_` = Self.(Self) -> Self}| (T, T) -> T
f|T| x, y: T = x + y - x
# T is a subtype of a nominal trait
# g: |T <: Add() and Sub()| (T, T) -> T
g|T| x, y: T = x + y - x
```

## Will Erg implement the ability to define its own operators?

A: There are no plans to do so. The main reason is that allowing the definition of custom operators raises the question of how to handle the concatenation order. Scala and Haskell, which allow the definition of custom operators, have different approaches, and this can be seen as evidence of a syntax that can lead to differences in interpretation. This can be seen as evidence of a syntax that can lead to differences in interpretation, and also has the disadvantage of creating code with low readability.

## Why did Erg do away with augmented assignment operators like `+=`?

First of all, variables are not mutable in Erg. In other words, reassignment is not possible. Once an object is assigned to a variable, it is bound to that variable forever until it is released out of scope. Once this is understood, the story is simple. For example, `i += 1` means `i = i + 1`, but such a construction is incorrect because variables are not reassignable. Another design principle of Erg is that operators have no side effects, and while Python generally does this, for some objects, such as Dict, the augmented assignment operator changes the internal state of the object. This is not a very beautiful design.
That is why augmented assignment operators have been deprecated in its entirety.

## Why does Erg give special grammatical treatment to objects with side effects?

Localization of side effects is an important part of code maintainability.

However, there are certainly ways to avoid giving side effects special linguistic treatment. For example, procedures can be substituted with algebraic effects (features on the type system).
But such congruence is not always correct. For example, Haskell did not treat strings as special, just arrays of characters, but this abstraction was wrong.

In what cases can we say that unification was wrong? One indicator is "does the congruence make the error message difficult to read?
The Erg designers decided that the error messages would be easier to read if side effects were treated specially.

Erg has a powerful type system, but it does not dominate everything with types.
If it did, it would end up the same way as Java, which tried to dominate everything with classes.
