# Pythonで実装されているモジュール

## [_erg_array.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_array.py)

Defines the `List` class, which is a wrapper for `list`.

## [_erg_bool.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_bool.py)

Defines the `Bool` class, which is a wrapper for `Nat` (note that it is not `bool`).

## [_erg_bytes.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_bytes.py)

## [_erg_control.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_control.py)

Defines functions that implement control structures such as `for!`, `if!`, etc.

## [_erg_converters.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_convertors.py)

Defines constructors for types like `int` and `str`. Currently, these constructors return `None` on failure.

## [_erg_dict.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_dict.py)

Defines the `Dict` class, which is a wrapper for `dict`.

## [_erg_float.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_float.py)

## [_erg_in_operator.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_in_operator.py)

Defines the implementation of the `in` operator. Erg's `in` operator adds type containment checks to the functionality of Python's `in` operator. For example, `1 in Int`, `[1, 2] in [Int; 2]` are possible.

## [_erg_int.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_int.py)

## [_erg_mutate.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_mutate_operator.py)

Defines the implementation of the `!` operator, which mutates objects. For example, it can convert `Int` to `IntMut` (`Int!`). This is essentially just calling the `mutate` method.

## [_erg_nat.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_nat.py)

## [_erg_range.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_range.py)

Defines range objects that appear like `1..3`. These are quite different from the `range` objects returned by Python's `range`, and are semantically closer to Rust's `Range`. They can be used with any sortable objects, not just `Int`.

## [_erg_result.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_result.py)

Defines the base class for errors, `Error`.

## [_erg_set.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_set.py)

## [_erg_std_prelude.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_std_prelude.py)

The entry point for the Erg runtime.

## [_erg_str.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_str.py)

