# `erg` build features

## debug

Enter debug mode. As a result, the behavior inside Erg is sequentially displayed in the log.
Independent of Rust's `debug_assertions` flag.

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

## pre-commit

Used to run tests in pre-commit. It's a bug workaround.

## large_thread

Increase the thread stack size. Used for Windows execution and test execution.
