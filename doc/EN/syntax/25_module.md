# module

Erg allows you to think of the file itself as a single record. This is called a module.

```python: foo.er
# foo.er
.i = 1
```

```python
# Defining the foo module is almost the same as defining this record
foo = {.i = 1}
```

```python,checker_ignore
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

Since module types are also record types, deconstruction assignment is possible.
For modules, you can omit the trailing `...`.

```python
# same as {sin; cos; ...} = import "math"
{sin; cos} = import "math"
```

## Module Visibility

Directories as well as files can be modules.
However, by default Erg does not recognize directories as Erg modules. To have it recognized, create a file named `__init__.er`.
`__init__.er` is similar to `__init__.py` in Python.

```console
└─┬ bar
  └─ __init__.er
```

Now the `bar` directory is recognized as a module. If the only file in `bar` is `__init__.er`, there is not much point in having a directory structure, but it is useful if you want to bundle several modules into a single module. For example:

```console
└─┬ bar
  ├─ __init__.er
  ├─ baz.er
  └─ qux.er
```

From outside the `bar` directory, you can use like the following.

```erg
bar = import "bar"

bar.baz.p!()
bar.qux.p!()
```

`__init__.er` is not just a marker that makes a directory as a module, it also controls the visibility of the module.

```erg
# __init__.er

# `. /` points to the current directory. It can be omitted
.baz = import ". /baz"
qux = import ". /qux"

.f x =
    .baz.f ...
.g x =
    qux.f ...
```

When you import a `bar` module from outside, the `baz` module will be accessible, but the `qux` module will not.

## circular dependencies

Erg allows you to define circular dependencies between modules.

```python
# foo.er
bar = import "bar"

print! bar.g 1
.f x = x
```

```python
# bar.er
foo = import "foo"

print! foo.f 1
.g x = x
```

However, variables created by procedure calls cannot be defined in circular reference modules.
This is because Erg rearranges the order of definitions according to dependencies.

```python,compile_fail
# foo.er
bar = import "bar"

print! bar.x
.x = g!(1) # ModuleError: variables created by procedure calls cannot be defined in circular reference modules
```

```python
# bar.er
foo = import "foo"

print! foo.x
.x = 0
```

In addition, An Erg module that is an entry point (i.e., a module that `__name__ == "__main__"`) cannot be the subject of circular references.

<p align='center'>
     <a href='./24_closure.md'>Previous</a> | <a href='./26_object_system.md'>Next</a>
</p>
