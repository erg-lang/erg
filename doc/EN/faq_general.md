# Erg FAQ

This FAQ is intended for the general Erg beginner.
For individual (common) technical issues, please refer to [here](./faq_technical.md) for individual (common) technical issues, and
[Here](./dev_guide/faq_syntax.md) for more information.

## What does it mean that Erg is a Python compatible language?

~~A: Erg's executable system, EVM (Erg VirtualMachine), executes Erg bytecode, which is an extension of Python bytecode. It introduces a static typing system and other features into the Python bytecode (such as introducing arguments to instructions that do not take arguments, and implementing unique instructions in the free numbers). This allows Erg to call Python code seamlessly and execute it fast.~~

A: Erg code is transpiled into Python bytecode. That is, it runs on the same interpreter as Python. Originally, we planned to develop a Cpython-compatible interpreter, and to combine it with the compiler to form "Erg". However, since the development of the processing system has lagged far behind that of the compiler, we have decided to release only the compiler in advance (But the interpreter is still under development).

## What languages have influenced Erg?

We have been influenced by more languages than we can count on both hands, but Python, Rust, Nim, and Haskell have been the strongest influences.
We inherited many semantics from Python, expression-oriented and trait from Rust, procedures from Nim, and functional programming-related features from Haskell.

## Languages that can call Python include Julia. Why did you create Erg?

A: One of the motivations for Erg's design was to have a language that is easy to use, yet has a powerful type system. That is, a language with type inference, Kind, dependent types, etc.
Julia can be typed, but it is really a dynamically typed language and does not have the compile-time error detection benefits of statically typed languages.

## Erg supports multiple styles of programming, including functional and object-oriented programming. Isn't this contrary to Python's "There should be one --and preferably only one-- obvious way to do it."?

A: In Erg, the term is taken in a more narrow context. For example, there are generally no aliases in the Erg API; Erg is "only one way" in this context.
In a larger context, such as FP or OOP, having only one way of doing things is not necessarily a convenience.
For example, JavaScript has several libraries to help create immutable programs, and C has several libraries for garbage collection.
However, having multiple libraries for even such basic features not only takes time to select, but also creates significant difficulties in integrating code that uses different libraries.
Even in Haskell, a purely functional language, there are libraries that support OOP.
If programmers don't have some stuffs, they will create them on their own. So, we think it would be better to provide them as a standard.
This also fits with Python's "Battery included" concept.
