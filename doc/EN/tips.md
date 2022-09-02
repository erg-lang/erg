# Tips

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tips.md%26commit_hash%3D020fa47edd39b86ed44bd8c46822aad6edf1442a)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tips.md&commit_hash=020fa47edd39b86ed44bd8c46822aad6edf1442a)

## Want to change the language in which errors are displayed

Please download Erg for your language.
However, external libraries may not support multiple languages.

## Want to change only certain attributes of a record

```erg
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; rest; ...} = record
mut_record = {.height = !height; ...rest}
```

## Want to shadow variables

Shadowing in the same scope is not possible with Erg. However, you can redefine them if the scope changes (This is a syntax called instance block).

````erg
## Get a T!-type object and finally assign it to a variable as type T
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
````

## Want to reuse a final class (non-inheritable class) somehow

You can create a wrapper class. This is a so-called composition pattern.

```erg
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## Want to use an enumerated type that is not a string

You can define a traditional enumerated type (algebraic data type) commonly found in other languages as follows
If you implement `Singleton`, classes and instances are identical.
Also, if you use `Enum`, the type of choice is automatically defined as a redirect attribute.

```erg
Ok = Class Impl := Singleton
Err = Class Impl := Singleton
ErrWithInfo = Inherit {info = Str}
Status = Enum Ok, Err, ErrWithInfo
stat: Status = Status.cons(ErrWithInfo) {info = "error caused by ..."}
match! stat:
    Status.Ok -> ...
    Status.Err -> ...
    Status.ErrWithInfo::{info} -> ...
```

```erg
Status = Enum Ok, Err, ErrWithInfo
# is equivalent to
Status = Class Ok or Err or ErrWithInfo
Status.
    Ok = Ok
    Err = Err
    ErrWithInfo = ErrWithInfo
```

## I want to enumerate at the beginning of 1

method 1:

```erg
arr = [...]
for! arr.iter().enumerate(start: 1), i =>
    ...
```

method 2:

```erg
arr = [...]
for! arr.iter().zip(1...) , i =>
    ...
```

## Want to test a (white box) non-public API

The private API in `foo.er` is specially accessible in the module `foo.test.er`.
The `foo.test.er` module cannot be imported, so it remains hidden.

```erg
# foo.er
private x = ...
```

```erg
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

```erg
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## Want the argument names to be identified on the type system

You can receive arguments by record.

```erg
Point = {x = Int; y = Int}

norm: Point -> Int
norm({x: Int; y: Int}): Int = x**2 + y**2
assert norm({x = 1; y = 2}) == norm({y = 2; x = 1})
```

## Want to stop warnings

There is no option in Erg to stop warnings (this is by design). Please rewrite your code.
