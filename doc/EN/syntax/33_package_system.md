# Package System

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/33_package_system.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/33_package_system.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Erg packages can be roughly classified into the app package, which is the application, and the lib package, which is the library.
The entry point of the app package is `src/app.er`. The `main` function defined in `app.er` is executed.
The entry point for the lib package is `src/lib.er`. Importing a package is equivalent to importing `lib.er`.

A package has a sub-structure called a module, which in Erg is an Erg file or directory composed of Erg files. External Erg files/directories are manipulatable objects as module objects.

In order for a directory to be recognized as a module, it is necessary to place a `(directory name).er` file in the directory.
This is similar to Python's `__init__.py`, but unlike `__init__.py`, it is placed outside the directory.

As an example, consider the following directory structure.

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

You can import `foo` and `bar` modules in `app.er`. The `bar` directory can be recognized as a module because of the `bar.er` file.
A `foo` module is a module consisting of files, and a `bar` module is a module consisting of directories. The `bar` module also contains `baz` and `qux` modules.
This module is simply an attribute of the `bar` module, and can be accessed from `app.er` as follows.

```erg
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

Note the `/` delimiter for accessing submodules. This is because there can be file names such as `bar.baz.er`.
Such filenames are discouraged, since the `.er` prefix is meaningful in Erg.
For example, a module for testing. A file ending with `.test.er` is a (white box) test module, which executes a subroutine decorated with `@Test` when the test is run.

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  └─ foo.test.er
./src

```erg
# app.er
foo = import "foo"

main args =
    ...
```

Also, files ending in ``.private.er`` are private modules and can only be accessed by modules in the same directory.

```console
└─┬
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.private.er
    └─ qux.er
```

```erg
# foo.er
bar = import "bar"
bar.qux
bar.baz # AttributeError: module 'baz' is private
```

```erg
# qux.er
baz = import "baz"
```

<p align='center'>
    <a href='./32_integration_with_Python.md'>Previous</a> | <a href='./34_generator.md'>Next</a>
</p>
