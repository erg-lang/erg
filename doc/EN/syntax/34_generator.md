# Generator

Generators are special procedures that use the `yield!` procedure in a block.

```python
g!() =
    yield! 1
    yield! 2
    yield! 3
```

`yield!` is a procedure defined in a block of subroutines that calls `self!.yield!`. Like `return`, it returns the value passed to it as a return value, but it has the feature of saving the current execution state of the block and executing it from the beginning when it is called again.
A generator is both a procedure and an iterator; a Python generator is a function that creates an iterator, while Erg iterates directly. Procedures themselves are generally not mutable objects (no `!`), but a generator is a mutable object because its own contents can change with each execution.

```python
# Generator!
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

A Python-style generator can be defined as follows.

```python
make_g() = () =>
    yield! 1
    yield! 2
    yield! 3
make_g: () => Generator!
```

<p align='center'>
    <a href='./33_package_system.md'>Previous</a> | Next
</p>
