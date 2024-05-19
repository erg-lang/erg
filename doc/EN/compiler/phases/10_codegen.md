# Code Generation

By default, Erg scripts are converted to pyc files and executed. In other words, they are executed as [Python bytecode](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/doc/JA/python/bytecode_instructions.md) rather than Python scripts.
The pyc files are generated from the HIR, which has been desugared (phase 8) and linked with dependencies (phase 9).
The process is handled by the [`PyCodeGenerator`](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/codegen.rs#L160). This structure takes `HIR` and returns a `CodeObj`.
The `CodeObj` corresponds to Python's Code object and contains the sequence of instructions to be executed, objects in the static area, and various other metadata. From the perspective of the Python interpreter, the `Code` object represents a scope. The `Code` representing the top-level scope will contain all the information necessary for execution. The `CodeObj` is serialized into a binary format using the [dump_as_pyc](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/ty/codeobj.rs#L378) method and written to a pyc file.

# Features Not Present in Python

## Erg Runtime

Erg runs on the Python interpreter, but there are various semantic differences from Python.
Some features are implemented by the compiler desugaring them into lower-level features, but some can only be implemented at runtime.

Examples include methods that do not exist in Python's built-in types.
Python's built-ins do not have a `Nat` type, nor do they have a `times!` method.
These methods are implemented by creating new types that wrap Python's built-in types.

These types are located [here](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std).
The generated bytecode first imports [`_erg_std_prelude.py`](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/codegen.rs#L3113). This module re-exports the types and functions provided by the Erg runtime.

## Record

Records are implemented using Python's `namedtuple`.

## Trait

Traits are implemented as Python's ABC (Abstract Base Classes).
However, Erg's traits have little meaning at runtime.

## match

Pattern matching is mostly reduced to a combination of type checks and assignment operations. This is done relatively early in the compilation process.