# Erg design's "Why" and Answers

## Erg memory management model

Use ownership in CPython backend + Python memory management model (though circular references in Erg code will not be handled by GC [see details](../syntax/18_ownership.md/#circular-references)

Using ownership + [Perceus](https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v1.pdf) memory in Erg's own virtual machine (Dyne) Management model, if Erg code uses the Python API then the Erg code uses the tracking garbage collection memory management model

In LLVM, WASM backend uses ownership + [Perceus](https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v1.pdf) memory management model

Regardless of the backend, the difference in memory management will not need any changes to the code

__Notice__:Erg's motivation for introducing an ownership system is not for "memory management without relying on GC" like Rust.
The aim of Erg's ownership system is ``localization of mutable state''. Erg has a notion of ownership attached to mutable objects.
This is because shared mutable state is prone to bugs and even violates type safety (see [here](../syntax/type/advanced/shared.md#SharedReference)). It's a judgmental decision.

## Why are the braces around type parameters || instead of <> or []?

This is because `<>` and `[]` cause syntax conflicts.

```python
# version []
id[T: Type] [t]: [T] = t
y = id[Int] # is this a function?
# <> version
id<T: Type> {t: T} = t
y = (id<Int, 1> 1) # Is this a tuple?
# {} version
id {T: Type} {t: T} = t
y = id{Int} # is this a function?
# || version
id|T: Type| t: T = t
y = id|Int| # OK
```

## The type of {i = 1} is {i = Int}, but in OCaml etc. it is {i: Int}. Why did Erg adopt the former syntax?

This is because Erg is designed so that the type itself can also be treated as a value.

```python
A = [Int; 3]
assert A[2] == Int
T = (Int, Str)
assert T.1 == Str
D = {Int: Str}
assert D[Int] == ​​Str
S = {.i = Int}
assert S.i == Int
```

## Any plans to implement macros in Erg?

Not currently. Macros have four main purposes. The first is compile-time computation. This is what compile-time functions do in Erg.
The second is code execution delays. This can be replaced with a do block. The third is the commonality of processing, for which polycorrelation and universal types are a better solution than macros. The fourth is automatic code generation, but this is not possible in Erg because it reduces readability.
Since the Erg type system takes over most of the functionality of macros, there is no motivation to introduce them.

## Why doesn't Erg have an exception mechanism?

Because in many cases error handling with the `Result` type is a better solution. The `Result` type is a common error handling technique used in relatively new programming languages.

In Erg, the `?` operator makes writing error-free.

```python
read_file!() =
    f = open!("foo.txt")? # Returns an error immediately if it fails, so f is of type File
    f.read_all!()

# Capturing like exceptions is also possible with the try procedure
try!:
    do!
        s = read_file!()?
        print!s
    e =>
        # block to execute when an error occurs
        print! e
        exit 1
```

When introducing Python functions, by default they are all assumed to be functions containing exceptions, with a return type of `Result`.
If you know it won't throw an exception, make it explicit with `assert`.

Another reason Erg does not introduce an exception mechanism is that it plans to introduce features for parallel programming.
This is because the exception mechanism is not compatible with parallel execution (it is troublesome to deal with cases such as when multiple exceptions occur due to parallel execution).

## Erg seems to eliminate Python features that are considered bad practice, why didn't you do away with inheritance?

This is because Python libraries have classes that are designed on the assumption that they will be inherited, and if inheritance is completely abolished, problems will arise in their operation.
However, in Erg, classes are final by default and multiple/multilevel inheritance is prohibited in principle, so inheritance can be used relatively safely.

## Why do polymorphic subtype inferences point to nominal traits by default?

Pointing to structural traits by default complicates typing and can introduce behavior unintended by the programmer.

```python
# If T is a subtype of a structural trait...
# f: |T <: Structural Trait {.`_+_` = Self.(Self) -> Self; .`_-_` = Self.(Self) -> Self}| (T, T) -> T.
f|T| x, y: T = x + y - x
# T is a subtype of a nominal trait
# g: |T <: Add() and Sub()| (T, T) -> T
g|T| x, y: T = x + y - x
```

## Will Erg not implement the ability to define its own operators?

A: There are no plans for that. The main reason is that allowing the definition of custom operators raises the question of what to do with their associativity. Scala and Haskell, which can define their own operators, handle them differently, but this can be seen as proof that the grammar can lead to different interpretations. Another disadvantage of custom operators is that they can create code that is not readable.

## Why did Erg deprecate extended assignment operators like +=?

First, there is no variable mutability in Erg. In other words, it cannot be reassigned. Once an object is bound to a variable, it remains bound to that variable until it goes out of scope and is freed. Mutability in Erg means object mutability. Once you know this, the story is easy. For example, `i += 1` means `i = i + 1`, but such syntax is illegal because variables are not reassigned. Another Erg design principle is that operators should not have side effects. Python is mostly like that, but for some objects such as Dict, extended assignment operators change the internal state of the object. This is not a very beautiful design.
That's why extended assignment operators are obsolete altogether.

## Why does Erg syntactically specialize objects with side effects?

Localizing side effects is an important aspect of code maintainability.

But there is certainly a way to get around side effects without linguistic specialization. For example, procedures can be substituted with algebraic effects (functions on the type system).
But such a union is not always correct. For example, Haskell treats strings as just arrays of characters without special treatment, but this abstraction is wrong.

In what cases could it be said that union was wrong? One indicator is "whether the unification makes the error message less readable".
The Erg designers decided that giving special treatment to side effects would make error messages easier to read.

Erg has a strong type system, but types don't rule everything.
If you do, you'll end up with the same fate that Java tried to rule everything with classes.