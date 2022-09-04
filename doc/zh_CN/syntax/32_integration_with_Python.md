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

```python
import foo

print(foo.public)
print(foo.private) # AttributeError:
```

## Import from Python

All objects imported from Python are by default of type `Object`. Since no comparisons can be made at this point, it is necessary to refine the type.

## Type Specification in the Standard Library

All APIs in the Python standard library are type specified by the Erg development team.

```python
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

```python
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```

```python
foo = pyimport "foo"
assert foo.bar(1) in Int
```

This ensures type safety by performing type checking at runtime. The ``declare`` function works roughly as follows.

```python
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
    <a href='./31_pipeline.md'>上一页</a> | <a href='./33_package_system.md'>下一页</a>
</p>
