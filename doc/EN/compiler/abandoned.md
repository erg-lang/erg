# Abandoned/rejected language specifications

## Overloading (ad-hoc polymorphism)

It was abandoned because it can be replaced by parametric + subtyping polymorphism, and it is incompatible with Python's semantics. See [overload](../syntax/type/advanced/overloading.md) article for details.

## Ownership system with explicit lifetime

It was planned to introduce an ownership system like Rust, but it was abandoned due to its incompatibility with Python's semantics and the need to introduce complicated specifications such as lifetime annotations, and all immutable objects are RC. Managed, mutable objects now have only one ownership.
Dyne does not have a GIL like C# and Nim, and the policy is to allow value objects and low-level operations within a safe range.