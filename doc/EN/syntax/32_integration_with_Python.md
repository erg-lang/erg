# Integration with Python

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/32_integration_with_Python.md%26commit_hash%3D7270b3f1541be0422fc46e1f533259738333c7d1)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/32_integration_with_Python.md&commit_hash=7270b3f1541be0422fc46e1f533259738333c7d1)

## Export to Python

When the Erg script is compiled, a .pyc file is generated, which can simply be imported as a Python module.
However, variables set to private on the Erg side cannot be accessed from Python.

```erg
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```

```console
erg --compile foo.er
```

```python
import foo

print(foo.public)
print(foo.private) # AttributeError:
```

## Import from Python

All objects imported from Python are by default of type `Object`. Since no comparisons can be made at this point, it is necessary to refine the type.

## Type Specification in the Standard Library

All APIs in the Python standard library are type specified by the Erg development team.

```erg
time = pyimport "time"
time.sleep! 1
```

## Type Specification for User Scripts

Create a `foo.d.er` file that types the Python `foo` module.
Type hints on the Python side are ignored since they are not 100% guaranteed.

```python
# foo.py
X = ...
def bar(x):
    ...
def baz():
    ...
...
```

```erg
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```

```erg
foo = pyimport "foo"
assert foo.bar(1) in Int
```

This ensures type safety by performing type checking at runtime. The ``declare`` function works roughly as follows.

```erg
declare|S: Subroutine| sub!: S, T =
    # Actually, => can be cast to a function without block side effects
    x =>
        assert x in T.Input
        y = sub!(x)
        assert y in T.Output
        y
```

Since this is a runtime overhead, a project is planned to statically type analyze Python scripts with Erg's type system.

<p align='center'>
    <a href='./31_pipeline.md'>Previous</a> | <a href='./33_package_system.md'>Next</a>
</p>
