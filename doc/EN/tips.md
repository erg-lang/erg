# Tips

## Want to change the language in which errors are displayed

Please download Erg for your language.
However, external libraries may not support multiple languages.

## Want to change only certain attributes of a record

```python
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; *rest} = record
mut_record = {.height = !height; *rest}
```

## Want to shadow variables

Shadowing in the same scope is not possible with Erg. However, you can redefine them if the scope changes (This is a syntax called instance block).

````python
## Get a T!-type object and finally assign it to a variable as type T
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
````

## Want to reuse a final class (non-inheritable class) somehow

You can create a wrapper class. This is a so-called composition pattern.

```python
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## Want to enumerate at the beginning of 1

method 1:

```python
arr = [...]
for! arr.iter().enumerate(start := 1), i =>
    ...
```

method 2:

```python
arr = [...]
for! arr.iter().zip(1..) , i =>
    ...
```

## Want to test a (white box) non-public API

The private API in `foo.er` is specially accessible in the module `foo.test.er`.
The `foo.test.er` module cannot be imported, so it remains hidden.

```python
# foo.er
private x = ...
```

```python
# foo.test.er
foo = import "foo"

@Test
'testing private' x =
    ...
    y = foo::private x
    ...
```

## Want to define a (variable) attribute that is read-only from the outside

You can make the attribute private and define a getter.

```python
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## When implementing a trait's methods, warnings are given for variables that were not used

You can use `discard` or `_ = ...`.

```python
T = Trait {.f = (Self, x: Int, s: Str) -> Int}

C = Class T
C|<: T|.
    f self, x, s =
        discard s # or _ = s
        ...
```

## Want to stop warnings

There is no option in Erg to stop warnings (this is by design). Please rewrite your code.
