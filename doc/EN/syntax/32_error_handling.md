# error handling system

Mainly use Result type.
In Erg, an error occurs if you throw away an Error type object (not supported at the top level).

## Exceptions, interop with Python

Erg does not have an exception mechanism (Exception). When importing a Python function

* Set return value to `T or Error` type
* `T or Panic` type (may cause runtime error)

There are two options, `pyimport` defaults to the latter. If you want to import as the former, use
Specify `Error` in `pyimport` `exception_type` (`exception_type: {Error, Panic}`).

## Exceptions and Result types

The `Result` type represents values ​​that may be errors. Error handling with `Result` is superior to the exception mechanism in several ways.
First of all, it's obvious from the type definition that the subroutine might throw an error, and it's also obvious when you actually use it.

```python,checker_ignore
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

In the above example, it is not possible to tell from this code alone which function raised the exception. Even going back to the function definition, it's hard to tell if the function throws an exception.

```python
# Erg
try!:
    do!:
        x = foo!()?.bar()
        y = baz!()
        qux!()?
    e =>
        print! e
```

On the other hand, in this example we can see that `foo!` and `qux!` can raise an error.
Precisely `y` could also be of type `Result`, but you'll have to deal with it eventually to use the value inside.

The benefits of using the `Result` type don't stop there. The `Result` type is also thread-safe. This means that error information can be (easily) passed between parallel executions.

## Context

Since the `Error`/`Result` type alone does not cause side effects, unlike exceptions, it cannot have information such as the sending location (Context), but if you use the `.context` method, you can put information in the `Error` object. can be added. The `.context` method is a type of method that consumes the `Error` object itself and creates a new `Error` object. They are chainable and can hold multiple contexts.

```python,chekcer_ignore
f() =
    todo() \
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

Note that `Error` attributes such as `.msg` and `.kind` are not secondary, so they are not context and cannot be overridden as they were originally created.

## Stack trace

The `Result` type is often used in other languages ​​because of its convenience, but it has the disadvantage of making it difficult to understand the source of an error compared to the exception mechanism.
Therefore, in Erg, the `Error` object has an attribute called `.stack`, and reproduces a pseudo-exception mechanism-like stack trace.
`.stack` is an array of caller objects. Each time an Error object is `returned` (including by `?`) it pushes its calling subroutine onto the `.stack`.
And if it is `?`ed or `.unwrap`ed in a context where `return` is not possible, it will panic with a traceback.

```python,checker_ignore
f x =
    ...
    y = foo.try_some(x)?
    ...

g x =
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
# ...
# Foo.try_some, line 10, file "foo.er"
# 10 | y = foo.try_some(x)?
# module::f, line 23, file "foo.er"
# 23 | y = f(x)?
# module::g, line 40, file "foo.er"
# 40 | i = g(1)?
# Error: ...
```

## Panic

Erg also has a mechanism for dealing with unrecoverable errors called __panicing__.
An unrecoverable error is an error caused by an external factor such as a software/hardware malfunction, an error so fatal that it makes no sense to continue executing the code, or an error unexpected by the programmer. Etc. If this happens, the program will be terminated immediately, because the programmer's efforts cannot restore normal operation. This is called "panicing".

Panic is done with the `panic` function.

```python,checker_ignore
panic "something went wrong!"
```

<p align='center'>
    <a href='./31_decorator.md'>Previous</a> | <a href='./33_pipeline.md'>Next</a>
</p>
