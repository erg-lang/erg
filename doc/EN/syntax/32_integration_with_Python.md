# Integration with Python

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
    <a href='./31_pipeline.md'></a> | <a href='./33_package_system.md'>Next</a>
</p>
