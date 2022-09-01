# Decorator

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_decorator.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_decorator.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Decorators are used to add or make explicit a specific state or behavior for a type or function.
The syntax for decorators is as follows.

```erg
@deco
X = ...
```

There can be more than one decorator, as long as they do not conflict.

A decorator is not a special object; its entity is simply a single-argument function. A decorator is equivalent to the following pseudo code.

```erg
X = ...
X = deco(X)
```

Since Erg does not allow reassignment, the above code will not pass, and a decorator is required.
Here are some frequently used built-in decorators.

## Inheritable

Indicates that the type being defined is an inheritable class. If the `scope` argument is set to `"public"`, the class can be inherited by classes in external modules. By default, it is `"private"` and cannot be inherited from outside.

## Final

disables overriding the `"final"` method. If you attach it to a class, it becomes a non-inheritable class, but since it is the default, it is meaningless.

## Override

Use to override an attribute, which by default Erg will fail if you try to define an attribute that is the same as the base class.

## Impl

Indicates implementation of traits.

```erg
ClosedAdd = Trait {
    . `_+_` = Self.(Self) -> Self
}
ClosedSub = Trait {
    . `_-_` = Self.(Self) -> Self
}

C = Class {i = Int}
C.
    @Impl ClosedAdd
    `_+_` self, other = C.new {i = self::i + other::i}
    @Impl ClosedSub
    `_-_` self, other = C.new {i = self::i - other::}
```

## Attach

Specifies the attachment patch that comes with the trait by default.
This allows you to reproduce the same behavior as the Rust trait.

```erg
# foo.er

Add R = Trait {
    .`_+_` = Self.(R) -> Self.AddO
    .AddO = Type
}
@Attach AddForInt, AddForOdd
ClosedAdd = Subsume Add(Self)

AddForInt = Patch(Int, Impl := ClosedAdd)
AddForInt.AddO = Int
AddForOdd = Patch(Odd, Impl := ClosedAdd)
AddForOdd.AddO = Even
```

This way, when you import traits from other modules, the attachment patch is automatically applied.

```erg
# Originally IntIsBinAdd, OddIsBinAdd must be imported at the same time, but can be omitted with attachment patch
{BinAdd; ...} = import "foo"

assert Int.AddO == Int
assert Odd.AddO == Even
```

Internally, they are only connected together using the trait's `.attach` method. If there is a conflict, it can be removed using the trait's `.detach` method.

```erg
@Attach X
T = Trait ...
assert X in T.attaches
U = T.detach(X).attach(Y)
assert X not in U.attaches
assert Y in U.attaches
```

## Deprecated

Indicates that the variable is outdated and deprecated.

## Test

Indicates a subroutine for tests. Test subroutines are executed with the `erg test` command.

<p align='center'>
    <a href='./28_spread_syntax.md'>Previous</a> | <a href='./30_error_handling.md'>Next</a>
</p>
