# Visibility

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/19_visibility.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/19_visibility.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Erg variables have the concept of __visibility__.
All variables we have seen so far are called __private variables__. These are variables that are invisible to the outside world.
For example, a private variable defined in the `foo` module cannot be referenced from another module.

```erg
# foo.er
x = "this is an invisible variable"
```

```erg
# bar.er
foo = import "foo"
foo.x # AttributeError: Module 'foo' has no attribute 'x' ('x' is private)
```

In contrast, there is also a __public variable__, which can be referenced externally.
Public variables are defined with `.`.

```erg
# foo.er
.x = "this is a visible variable"
```

```erg
# bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

Private variables do not need to be marked with anything, but can be marked with `::` or `self::` (or `Self::` for types, etc.) to make them explicitly private. A module can also be `module::`.

```erg
::x = "this is an invisible variable"
assert ::x == x
assert self::x == ::x
assert module::x == ::x
```

In the context of mere sequential execution, private variables are almost synonymous with local variables. They can be referenced from inner scope.

```erg
::x = "this is a private variable"
y =
    x + 1 # exactly module::x
```

The `::` allows you to distinguish between variables with the same name in a scope.
Specify the scope of the variable you want to reference on the left. For the top level, specify `module`.
If not specified, the innermost variable is referenced as in the normal case.

```erg
::x = 0
assert x == 0
y =
    ::x = 1
    assert x == 1
    z =
        ::x = 2
        assert ::x == 2
        assert z::x == 2
        assert y::x == 1
        assert module::x == 0
```

In the scope of an anonymous subroutine, `self` specifies its own scope.

```erg
x = 0
f = x ->
    log module::x, self::x
f 1 # 0 1
```

`::` is also responsible for accessing private instance attributes.

```erg
x = 0
C = Class {x = Int}
C.
    # Top-level x is referenced (warns to make it module::x)
    f1 self = x
    # x of instance attribute is referenced
    f2 self = self::x
```

## Visibility in external modules

A class defined in one module can actually define methods from an external module as well.

```erg
## foo.er
.Foo = Class()
```

```erg
# bar.er
{Foo; ...} = import "foo"

Foo::
    private self = pass
Foo.
    public self = self::private()

.f() =
    foo = Foo.new()
    foo.public()
    foo::private() # AttributeError
```

However, both of those methods can only be used within that module.
Externally defined private methods can be referenced by methods of the `Foo` class only within the defining module.
Public methods are exposed outside the class, but not to outside the module.

```erg
# baz.er.
{Foo; ...} = import "foo"

foo = Foo.new()
foo.public() # AttributeError: 'Foo' has no attribute 'public' ('public' is defined in module 'bar')
```

Also, you cannot define a method on the type you are re-exporting.
This is to avoid confusion when a method is found or not found depending on the module from which it is imported.

```erg
# bar.er
{.Foo; ...} = import "foo"

.Foo::
    private self = pass # Error
.Foo.
    Foo:: public self = self::private() # Error
```

If you want to do something like this, define a [patch](./type/07_patch.md).

```erg
# bar.er.
{Foo; ...} = import "foo"

FooImpl = Patch Foo
FooImpl::
    private self = pass
FooImpl.
    public self = self::private()
```

```erg
# baz.er
{Foo; ...} = import "foo"
{FooImpl; ...} = import "bar"

foo = Foo.new()
foo.public()
```

## Restricted Public Variables

Variable visibility is not limited to complete public/private.
You can also publish with restrictions.

``` erg
# foo.er
.record = {
     .a = {
         .(.record)x = 0
         .(module)y = 0
         .z = 0
     }
     _ = .a.x # OK
     _ = .a.y # OK
     _ = .a.z # OK
}

_ = .record.a.x # VisibilityError
_ = .record.a.y # OK
_ = .record.a.z # OK
```

``` erg
foo = import "foo"
_ = foo.record.a.x # VisibilityError
_ = foo.record.a.y # VisibilityError
_ = foo.record.a.z # OK
```

<p align='center'>
     <a href='./18_ownership.md'>Previous</a> | <a href='./20_naming_rule.md'>Next</a>
</p>
