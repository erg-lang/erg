# Integration with Python

## Export to Python

When the Erg script is compiled, a .pyc file is generated, which can simply be imported as a Python module.
However, variables set to private on the Erg side cannot be accessed from Python.

```python
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```

```console
erg --compile foo.er
```

```python,checker_ignore
import foo

print(foo.public)
print(foo.private) # AttributeError:
```

## import from Python

By default, all objects imported from Python are of type `Object`. Since no comparison is possible with this type, it is necessary to narrow down the type.

## Type specification in the standard library

All APIs in the Python standard library are type-specified by the Erg development team.

```python
time = pyimport "time"
time.sleep! 1
```

## Type specification for user scripts

Type hints on the Python side are ignored. Create a `foo.d.er` file that types the Python `foo` module.

```python
# foo.py
X = ...
def bar(x):
    ...
def baz():
    ...
class C:
    ...
...
```

```python
# foo.d.er
.X: Int
.bar!: Int => Int
.foo! = baz!: () => Int # aliasing
.C!: Class
```

No syntax other than declarations and definitions (aliasing) are allowed in ``d.er``.

If an identifier on the Python side is not a valid identifier in Erg, it can be escaped by enclosing it in single quotes (`'`).

## Overloading

A special type that can be used only with Python typing is the overloaded type. This is a type that can accept multiple types.

```python
f: (Int -> Str) and (Str -> Int)
```

Overloaded types can be declared by taking a subroutine type intersection (`and`, not union `or`).

This allows you to declare a function whose return type depends on the type of its arguments.

```python
f(1): Str
f("1"): Int
```

The type decisions are collated from left to right, and the first match is applied.

Such polymorphism is called ad hoc polymorphism and is different from Erg's polymorphism, which uses type variables and trait bounds. Ad hoc polymorphism is generally discouraged, but it is a necessary  because of its universal use in Python code.

Parameter types of overloaded types may be in a subtype relationship and may have different number of parameters, but they must not be of the same type, i.e. return type overload is not allowed.

```python
# OK
f: (Nat -> Str) and (Int -> Int)
f: ((Int, Int) -> Str) and (Int -> Int)
```

```python,compile_fail
# NG
f: (Int -> Str) and (Int -> Int)
```

## Declaration of Trait Implementation

To implement a trait and declare trait members for a class, write the following (taken from [type declarations for numpy.NDArray](https://github.com/erg-lang/erg/blob/main/crates/erg_compiler/lib/external/numpy.d/__init__.d.er)).

```erg
.NDArray = 'ndarray': (T: Type, Shape: [Nat; _]) -> ClassType
...
.NDArray(T, S)|<: Add .NDArray(T, S)|.
    Output: {.NDArray(T, S)}
    __add__: (self: .NDArray(T, S), other: .NDArray(T, S)) -> .NDArray(T, S)
```

## Notes

Currently, Erg unconditionally trusts the contents of type declarations. In other words, you can declare a variable of type `Str` even if it is actually a variable of type `Int`, or declare a subroutine as a function even if it has side effects, etc.

Also, it is troublesome that type declarations cannot be omitted even for trivial code, so the [Project for static type analysis of Python scripts with Erg's type system](https://github.com/mtshiba/pylyzer) is underway.

<p align='center'>
    <a href='./33_pipeline.md'>Previous</a> | <a href='./35_package_system.md'>Next</a>
</p>
