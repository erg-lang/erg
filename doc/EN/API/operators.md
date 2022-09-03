# operator

## infix operator

### `_+_`|R; O; A <: Add(R, O)|(x: A, y: R) -> O

Perform addition.

### `_-_`|R; O; S <: Sub(R, O)|(x: S, y: R) -> O

Perform subtraction.

### `*`|R; O; M <: Mul R, O|(x: M, y: R) -> O

Perform multiplication.

### `/`|R; O; D <: Div(R, O)|(x: D, y: R) -> O

Perform division.

## infix alphabet operator

### `and`(x: Bool, y: Bool) -> Bool

Executes the and operation.

### `or`(x: Bool, y: Bool) -> Bool

Executes the and operation.

## prefix operator

### `+_`|T <: Num|(x: T) -> T

Same as id by default.

### `-_`|T <: Num|(x: T) -> T.Neg

For example, Nat.`-`: Nat -> Neg and the return value is different.

### `!`|T <: Immut|(x: T) -> `T!`

Create a mutable object from an immutable object.
This operator itself is not procedural and can be used inside a function.

### `..`|T <: Ord|(x: T) -> Range T

Creates a Range object with no lower bound at the end of x.
x..x returns only x as an iterator.

### `..<`|T <: Ord|(x: T) -> Range T

x..<x results in an empty Range object, yielding nothing as an iterator.

## postfix operator

A postfix operator is called when parsing a parsing infix operator fails.
That is, even if `x..` returns a function, `x..y` is `(..)(x, y)` and not `(x..)(y)`.

### |T <: Ord|(x: T)`..` -> Range T

Creates a Range object with no upper bound starting at x.

### |T <: Ord|(x: T)`<..` -> Range T