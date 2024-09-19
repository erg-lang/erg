# `erg` build features

## debug

Enter debug mode. As a result, the behavior inside Erg is sequentially displayed in the log. Also, enable `backtrace_on_stack_overflow`.
Independent of Rust's `debug_assertions` flag.

## backtrace

Enable only `backtrace_on_stack_overflow`.

## japanese

Set the system language to Japanese.
Erg internal options, help (help, copyright, license, etc.) and error display are guaranteed to be Japanese.

## simplified_chinese

Set the system language to Simplified Chinese.
Erg internal options, help (help, copyright, license, etc.) and errors are displayed in Simplified Chinese.

## traditional_chinese

Set the system language to Traditional Chinese.
Erg internal options, help (help, copyright, license, etc.) and errors are displayed in Traditional Chinese.

## unicode/pretty

The compiler makes the display rich.

## large_thread

Increase the thread stack size. Used for Windows execution and test execution.

## els

`--language-server` option becomes available.
`erg --language-server` will start the Erg language server.

## py_compatible

Enable Python-compatible mode, which makes parts of the APIs and syntax compatible with Python. Used for [pylyzer](https://github.com/mtshiba/pylyzer).

## experimental

Enable experimental features (contains `parallel`).

## log-level-error

Only display error logs.

## parallel

Enable compiler parallelization. Unstable feature.
