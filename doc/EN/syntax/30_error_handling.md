# Error Handling

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/30_error_handling.md%26commit_hash%3D6dc8c5015b6120497a26d80eaef65d23eb2bee2a)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/30_error_handling.md&commit_hash=6dc8c5015b6120497a26d80eaef65d23eb2bee2a)

Mainly uses Result type.
Erg will throw away Error type objects (not supported at top level).

## Exceptions, Interoperation with Python

Erg does not have an exception mechanism (Exception).

* Set the return value as `T or Error` type.
* set the return value to `T or Panic` type (which may raise an error at runtime).

The latter is the default for `pyimport`. If you want to import as the former, use
If you want to import as the former, specify `Error` in `pyimport`'s `exception_type` (`exception_type: {Error, Panic}`).

## Exceptions and Result types

The `Result` type represents a value that may be an error. Error handling with `Result` is superior to the exception mechanism in several ways.
First, you can tell from the type definition that a subroutine may raise an error, and it is obvious when you actually use it.

```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

In the above example, it is not clear from the code alone which function sends the exception. Even going back to the function definition, it is difficult to determine if the function raises the exception.

```erg
# Erg
try!
    do!
        x = foo!()? .bar()?
        y = baz!
        qux!()?
    e =>
        print!
```

On the flip side, we can see that `foo!` and `qux!` can produce errors in this example.
To be precise, `y` could also be of type `Result`, but we will have to deal with that eventually in order to use the values inside.

That is not the only advantage of using the `Result` type. The `Result` type is also thread-safe. This means that error information can be passed around (easily) during parallel execution.

## Context

Unlike exceptions, the `Error`/`Result` types by themselves do not have side-effects, so they do not have context, but the `.context` method can be used to add information to the `Error` object. The `.context` method is a type of method that creates a new `Error` object by consuming the `Error` object itself. It is chainable and can hold multiple contexts.

```erg
f() =
    todo() \f}
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

Note that `Error` attributes such as `.msg`, `.kind`, etc., are not secondary and are not context and cannot be overwritten as they were when they were first generated.

## Stack Trace

The `Result` type has been adopted by many other languages because of its convenience, but it has the disadvantage that the source of the error is harder to identify than the exception mechanism.
Therefore, Erg has an attribute `.stack` on the `Error` object to reproduce a pseudo-exception mechanism-like stack trace.

`.stack` is an array of caller objects. Each time an Error object is `return`ed (including by `?`) it stacks its calling subroutine on the `.stack`.
And if it is `?`ed or `.unwrap`ed in a context where `return` is not possible, it will panic with a traceback.

```erg
f x =
    ...
    y = foo.try_some(x)?
    ...

g x = ...
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
# ...
# Foo.try_some, line 10, file "foo.er"
# 10 | y = foo.try_some(x)?
# module::f, line 23, file "foo.er"
# 23 | y = f(x)?
# module::g, line 40, file "foo.er"?
# 40 | i = g(1)?
# Error: ...
```

## Panic

Erg also has a mechanism called __panicking__ to deal with unrecoverable errors.
Unrecoverable errors are errors caused by external factors such as software/hardware malfunctions, errors that are so fatal that it makes no sense to continue executing the code, or errors that the programmer did not anticipate. When such an error occurs, the program is terminated on the spot because it cannot be restored to the normal system through the programmer's efforts. This is called "panicking".

Panicking is done with the `panic` function.

```erg
panic "something went wrong!"
```

<p align='center'>
    <a href='./29_decorator.md'>Previous</a> | <a href='./31_pipeline.md'>Next</a>
</p>
