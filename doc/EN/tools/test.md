# test subcommand

The erg command has a subcommand called test, which supports test implementation and execution.

## Test decorator (@Test)

Erg tests the `@Test` subroutine in the `tests` directory in the package or in the `*.test.er` file with the `erg test` command.
`tests` subroutines are in charge of black-box testing (not testing private functions), and `*.test.er` subroutines are in charge of white-box testing (testing private functions as well).

```python
# tests/test1.er
{add; ...} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

The execution result is displayed as a summary and can be output in various file formats (.md, .csv, etc.).

## Doc Test

In Erg, `#` and `#[` are comment lines, but `##` and `#[[` are doc comments, and comments can be displayed as markdown from editors such as VSCode.
Furthermore, the source code in the doc comment is automatically tested with the erg test command if erg is specified.
Below is an example test.

```python
VMs =...
    ...
    #[[
    execute commands.
    ```erg
    # VM in standard configuration
    {vm1; ...} = import "tests/mock"

    assert vm1.exec!("i = 0") == None
    assert vm1.exec!("i").try_into(Int)? == 0
    ```
    ]]#
    .exec! ref self, src =
        ...
    ...
```

Mock objects (mock objects) used for testing are defined in the `tests/mock` module.