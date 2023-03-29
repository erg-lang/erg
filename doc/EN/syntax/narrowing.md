# Type Narrowing

Erg allows type narrowing by conditional branching.

```python,compile_fail
x: Int or NoneType
y = x + 1 # TypeError
```

The type of `x` is `Int or NoneType`. Because it may be `None`, ``x + 1`` will cause a type error.

```python
if x != None, do:
    x + 1 # OK
    ...
```

However, by checking the conditional branch to make sure that `x` is not `None`, as above, the type of `x` is narrowed down to `Int`.
The `isinstance` function does the same thing.

```python
if isinstance(x, Int), do:
    x + 1 # OK
    ...
```

## Subroutines and operators that cause the narrowing effect

Currently, only the following subroutines and operators can cause the narrowing effect.

### `in`

The expression `x in T` determines if `x` is an instance of `T`.

```python
x: Int or Str
if x in Int, do:
    x + 1 # OK
    ...
```

### `notin`

Has the opposite meaning of `in`.

### `isinstance`

Similar to `x in T`, but only if the type is a simple class.

```python
x in 1.. # OK
isinstance(x, 1..) # TypeError
isinstance(x, Int) # OK
```

### `==`/`is!`

The expressions `x == y` or `x is! y` determine whether `x` is equal to `y` (see the API documentation for the difference between the two).

### `!=`/`isnot!`

The opposite of `==`/`is!`.

### `>=`/`>`/`<=`/`<`

Refinement type methods may be used.

```python
i: Int
if i >= 0, do:
    log i.times! # <bound method ... >
```

## Subroutines that consume the narrowing effect

`if/if!/while!` causes narrowing only within the block passed as argument.
If you exit the scope, the refinement is removed.
For `assert`, narrowing occurs only within the block after the `assert` call.

### `if`/`if!`

```python
x: Int or Str
if x in Int, do:
    x + 1 # OK
    ...
```

### `while!`

```python
x: Int! or NoneType
while! do x != None, do!:
    x.inc!() # OK
    ...
```

### `assert`

```python
x: Int or NoneType
assert x != None
x: Int
```
