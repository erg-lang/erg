# Package System

Erg packages can be roughly classified into the app package, which is the application, and the lib package, which is the library.
The entry point of the app package is `src/app.er`. The `main` function defined in `app.er` is executed.
The entry point for the lib package is `src/lib.er`. Importing a package is equivalent to importing `lib.er`.

A package has a sub-structure called a module, which in Erg is an Erg file or directory composed of Erg files. External Erg files/directories are manipulatable objects as module objects.

In order for a directory to be recognized as a module, a `__init__.er` file must be placed in the directory.
This is similar to `__init__.py` in Python.

As an example, consider the following directory structure.

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  └─┬ bar
    ├─ __init__.er
    ├─ baz.er
    └─ qux.er
```

In `app.er` you can import `foo` and `bar` modules. The `bar` directory can be recognized as a module because of the `__init__.er` file.
A `foo` module is a module consisting of files, and a `bar` module is a module consisting of directories. The `bar` module also contains `baz` and `qux` modules.
This module is simply an attribute of the `bar` module, and can be accessed from `app.er` as follows

```python
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

Note that the delimiter for accessing submodules is `/`. This is because a file name like `bar.baz.er` is possible.
However, such filenames are discouraged, because in Erg, the identifier immediately preceding the `.er`, the prefix, is meaningful.
For example, a module for testing. A file ending with `.test.er` is a (white box) test module, which executes a subroutine decorated with `@Test` when the test is run.

```console
└─┬ . /src
  ├─ app.er
  ├─ foo.er
  └─ foo.test.er
```

```python
# app.er
foo = import "foo"

main args =
    ...
```

Also, modules that are not reimported in `__init__.er` are private modules and can only be accessed by modules in the same directory.

```console
└─┬
  ├─ foo.er
  └─┬ bar
    ├─ __init__.er
    ├─ baz.er
    └─ qux.er
```

```python
# __init__.py
.qux = import "qux" # this is public
```

```python,checker_ignore
# foo.er
bar = import "bar"
bar.qux
bar.baz # AttributeError: module 'baz' is private
```

```python
# qux.er
baz = import "baz"
```

<p align='center'>
    <a href='./34_integration_with_Python.md'>Previous</a> | <a href='./36_generator.md'>Next</a>
</p>
