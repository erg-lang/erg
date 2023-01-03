# test

Testing is an important part of ensuring code quality.

Execute the test with the following command.

``` sh
cargo test --features large_thread
```

Since cargo takes a small thread for running tests, we use the `large_thread` flag to avoid stack overflows.

## Placement of tests

Arrange them according to the implemented features. Place parser tests under `erg_parser/tests`, compiler (type checker, etc.) tests under `erg_compiler/tests`, language feature tests that users can use directly under `erg/tests` (However, the tests are currently in development and are not necessarily arranged according to this convention).

## How to write tests

There are two types of tests. A positive test and a negative test.
A positive test is a test to check whether the compiler operates as intended, and a negative test is a test to check whether the compiler properly outputs an error for invalid input.
Due to the nature of programming language processors, among all software, they are particularly susceptible to invalid input, and errors must always be presented to the user, so the latter must also be taken care of.

If you add a new feature to the language, you need to write at least one positive test. Also, please write negative tests if possible.
