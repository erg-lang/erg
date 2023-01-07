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

Create a `foo.d.er` file that types the Python `foo` module.
Type hints on the Python side are ignored since they are not 100% guaranteed.

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

Note that all Python functions can only be registered as procedures, and all classes as variable classes.

```python
foo = pyimport "foo"
assert foo.bar!(1) in Int
```

This ensures type safety by performing type checking at runtime. The checking mechanism generally works as follows.

```python
decl_proc proc!: Proc, T =
    x =>
        assert x in T.Input
        y = proc!(x)
        assert y in T.Output
        y
```

Since this is a runtime overhead, a project is planned to statically type analyze Python scripts with Erg's type system.

<p align='center'>
    <a href='./32_pipeline.md'>Previous</a> | <a href='./34_package_system.md'>Next</a>
</p>
