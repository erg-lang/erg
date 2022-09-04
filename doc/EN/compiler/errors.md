# Erg Compiler Errors

## AssignError

Raised when attempting to rewrite an immutable variable.

## AttributeError

Raised when trying to access an attribute that does not exist.

## PurityError

Raised when writing code that causes side effects in scopes (functions, immutable types, etc.) where side effects are not allowed.

## MoveError

Raised when you try to access a variable that has already been moved.

## BorrowError

Raised when an attempt is made to obtain another variable reference while a borrow exists for an object.

## CyclicError

Raised when there is an apparent non-stop cycle.

```python
i: Int = i

f(): Int = g()
g() = f()

h(): Int = module::h()

T = U
U = T
```

## BytecodeError

Raised if the bytecode read is corrupt.

## CompileSystemError

Raised when an error occurs inside the compiler.

## EnvironmentError

Raised when there is no access permission during installation.

## FeatureError

Raised when an experimental feature that is not officially provided is detected.

## ImportError

## IndentationError

Raised when an invalid indentation is detected.
Derived from SyntaxError.

## NameError

Raised when accessing a variable that does not exist.

## NotImplementedError

Raised when calling an API that has a definition but no implementation.
Derived from TypeError.

## PatternError

Raised when an invalid pattern is detected.
Derived from SyntaxError.

## SyntaxError

Raised when an invalid syntax is detected.

## TabError

Raised when a tab character is used for indentation/space.
Derived from SyntaxError.

## TypeError

Raised when the object type does not match.

## UnboundLocalError

Raised when a variable is used before it is defined.
More precisely, it occurs when a variable defined in a scope is used before it is defined.

```python
i = 0
f x =
    y = i + x
    i = 1
    y + i
```

In this code, the `i` in `y = i + x` is an undefined variable.
However, if it is a constant, it can be called in another function before it is defined.

```python
f() = g()
g() = f()
```

## Erg Compiler Warnings

## SyntaxWarning

This happens when syntactically sound but redundant or uncommon code is detected (e.g., unnecessary `()`).

```python
if (True): # SyntaxWarning: unnecessary parentheses
    ...
```

## DeprecationWarning

Raised if the referenced object is deprecated.
(Developers should always provide an alternative Hint when raising this Warning.)

## FutureWarning

Raised when code is detected that may cause problems in the future.
This warning is caused by version compatibility issues (including libraries) or changes in syntax or API.

## ImportWarning
