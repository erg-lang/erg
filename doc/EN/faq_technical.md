# Technical FAQ

This section answers technical questions about using the Erg language. In other words, it contains questions that begin with What or Which, and questions that can be answered with Yes/No.

For more information on how the grammar was determined, see [here](./faq_syntax.md) for the underlying syntax decisions, and [here](./faq_general.md).

## Is there an exception mechanism in Erg?

A: No. Erg uses the `Result` type instead. See [here](./faq_syntax.md) for why Erg does not have an exception mechanism.

## Does Erg have a type equivalent to TypeScript's `Any`?

A: No, there is not. All objects belong to at least the `Object` class, but this type only provides a minimal set of attributes, so you can't do whatever you want with it like you can with Any.
The `Object` class is converted to the desired type through dynamic inspection by `match`, etc. It is the same kind of `Object` in Java and other languages.
In the Erg world, there is no chaos and hopelessness like in TypeScript, where the API definition is ``Any''.

## What is the difference between Never, {}, None, (), NotImplemented, and Ellipsis?

A: `Never` is an "impossible" type. A subroutine that produces a runtime error has `Never` (or a merger type of `Never`) as its return type. The program will stop as soon as it detects this. Although the `Never` type is by definition also a subclass of all types, `Never` type objects never appear in Erg code and are never created. `{}` is equivalent to `Never`.
`Ellipsis` is an object that represents an ellipsis, and comes from Python.
`NotImplemented` is also from Python. It is used as a marker for not implemented, but Erg prefers the `todo` function which produces an error.
`None` is an instance of `NoneType`. It is often used with the `Option` type.
`()` is a unit type and an instance of itself. It is used when you want to return a "meaningless value" such as the return value of a procedure.

## Why is `x = p!()` valid but `f() = p!()` causes an EffectError?

`!` is not a marker for the product of a side-effect, but for an object that can cause a side-effect.
Procedure `p!` and mutable type `T!` can cause side effects, but if the return value of `p!()`, for example, is of type `Int`, it itself no longer causes side effects.

## When I try to use the Python API, I get a type error in Erg for code that was valid in Python. What does this mean?

A: The Erg API is typed as closely as possible to the Python API specification, but there are some cases that cannot be fully expressed.
Also, input that is valid according to the specification but deemed undesirable (for example, inputting a float when an int should be inputted) may be treated as a type error at the discretion of the Erg development team.
