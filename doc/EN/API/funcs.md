# functions

## basic functions

### if|T; U|(cond: Bool, then: T, else: U) -> T or U

### map|T; U|(i: Iterable T, f: T -> U) -> Map U

Note that the order of arguments is reversed from Python.

### log(x: Object, type: LogType = Info) -> None

Log `x` in debug display. Logs are summarized and displayed after the execution is finished.
Emoji-capable terminals are prefixed according to `type`.

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

### panic(msg: Str) -> Panic

Display msg and stop.
Emoji-capable terminals have a 🚨 prefix.

### discard|T|(x: ...T) -> NoneType

Throw away `x`. Used when the return value is not used. Unlike `del`, it does not make the variable `x` inaccessible.

```python
p!x=
    # Let q! return some None or non-() value
    # use `discard` if you don't need it
    discard q!(x)
    f x

discard True
assert True # OK
```

### import(path: Path) -> Module or CompilerPanic

Import a module. Raises a compilation error if the module is not found.

### eval(code: Str) -> Object

Evaluate code as code and return.

### classof(object: Object) -> Class

Returns the class of `object`.
However, since classes cannot be compared, use `object in Class` instead of `classof(object) == Class` if you want to judge instances.
The structure type determined at compile time is obtained with `Typeof`.

## Iterator, Array generation system

### repeat|T|(x: T) -> RepeatIterator T

```python
rep = repeat 1 # Repeater(1)
for! rep, i =>
    print!i
# 1 1 1 1 1 ...
```

### dup|T; N|(x: T, N: Nat) -> [T; N]

```python
[a, b, c] = dup new(), 3
print! a # <Object object>
print! a == b # False
```

### cycle|T|(it: Iterable T) -> CycleIterator T

```python
cycle([0, 1]).take 4 # [0, 1, 0, 1]
cycle("hello").take 3 # "hellohellohello"
```

## constant expression functions

### Class

Create a new class. Unlike `Inherit`, passing through `Class` is independent of the base type and methods are lost.
You won't be able to compare, but you can do things like pattern matching.

```python
C = Class {i = Int}
NewInt = ClassInt
Months = Class 1..12
jan = Months.new(1)
jan + Months.new(2) # TypeError: `+` is not implemented for 'Months'
match jan:
    1 -> log "January"
    _ -> log "Other"
```

The second argument, Impl, is the trait to implement.

### Inherit

Inherit a class. You can use the base class methods as they are.

### Traits

Create a new trait. Currently, only record types can be specified.

### Type of

Returns the argument type. Use `classof` if you want to get the runtime class.
If you use it for type specification, Warning will appear.

```python
x: Type of i = ...
# TypeWarning: Typeof(i) == Int, please replace it
```

### Deprecated

Use as a decorator. Warn about deprecated types and functions.