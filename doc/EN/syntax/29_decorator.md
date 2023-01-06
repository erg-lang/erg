# Decorator (modifier)

Decorators are used to add or demonstrate a particular state or behavior to a type or function.
The syntax of the decorator is as follows.

```python
@deco
X = ...
```

You can have multiple decorators as long as they don't conflict.

A decorator is not a special object, it's just a one-argument function. The decorator is equivalent to the following pseudocode.

```python
X = ...
X = deco(X)
```

Erg doesn't allow reassignment of variables, so code like the one above won't work.
For simple variables it's the same as `X = deco(...)`, but for instant blocks and subroutines you can't do that, so you need a decorator.

```python
@deco
f x =
    y = ...
    x + y

# You can also prevent the code from becoming horizontal
@LongNameDeco1
@LongNameDeco2
C = Class...
```

Below are some frequently used built-in decorators.

## Inheritable

Indicates that the defining type is an inheritable class. If you specify `"public"` for the argument `scope`, it will be possible to inherit even the class of the external module. By default it is `"private"` and cannot be inherited externally.

## Final

Make the method non-overridable. Adding it to a class makes it a non-inheritable class, but since it's the default it doesn't make sense.

## Override

Used when overriding attributes. By default, Erg will throw an error if you try to define the same attribute as the base class.

## Impl

Indicates that the argument trait is implemented.

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
Sub = Trait {
    .`_-_` = Self.(Self) -> Self
}

C = Class({i = Int}, Impl := Add and Sub)
C.
    @Impl Add
    `_+_` self, other = C.new {i = self::i + other::i}
    @Impl Sub
    `_-_` self, other = C.new {i = self::i - other::}
```

## Attach

Specifies the attachment patch that comes with the trait by default.
This allows you to reproduce the same behavior as Rust traits.

```python
# foo.er
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
@Attach AddForInt, AddForOdd
ClosedAdd = Subsume Add(Self)

AddForInt = Patch(Int, Impl := ClosedAdd)
AddForInt.AddO = Int
AddForOdd = Patch(Odd, Impl := ClosedAdd)
AddForOdd.AddO = Even
```

This will automatically apply the attachment patch when importing traits from other modules.

```python
# Originally, IntIsBinAdd and OddIsBinAdd should be imported at the same time, but if it's an attachment patch, you can omit it
{BinAdd; ...} = import "foo"

assert Int. AddO == Int
assert Odd.AddO == Even
```

Internally it's just attached using the trait's `.attach` method. Conflicts can be removed with the trait's `.detach` method.

```python
@Attach X
T = Trait ...
assert X in T. attaches
U = T.detach(X).attach(Y)
assert X not in U. attaches
assert Y in U. attaches
```

## Deprecated

Indicates that the variable specification is obsolete and deprecated.

## Test

Indicates that this is a test subroutine. Test subroutines are run with the `erg test` command.

<p align='center'>
    <a href='./28_spread_syntax.md'>Previous</a> | <a href='./30_error_handling.md'>Next</a>
</p>
