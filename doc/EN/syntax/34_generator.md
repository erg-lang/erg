# Generator

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_generator.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_generator.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Generators are special procedures that use the `yield!` procedure in a block.

```erg
g!() =
    yield! 1
    yield! 2
    yield! 3
```

`yield!` is a procedure defined in a block of subroutines that calls `self!.yield!`. Like `return`, it returns the value passed to it as a return value, but it has the feature of saving the current execution state of the block and executing it from the beginning when it is called again.
A generator is both a procedure and an iterator; a Python generator is a function that creates an iterator, while Erg iterates directly. Procedures themselves are generally not mutable objects (no `!`), but a generator is a mutable object because its own contents can change with each execution.

```erg
# Generator!
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

A Python-style generator can be defined as follows.

```erg
make_g() = () =>
    yield! 1
    yield! 2
    yield! 3
make_g: () => Generator!
```

<p align='center'>
    <a href='./33_package_system.md'>Previous</a> | Next
</p>
