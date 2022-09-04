# Visibility

Erg variables have the concept of __visibility__.
All the variables we've seen so far are called __private variables__. This is an externally invisible variable.
For example, a private variable defined in the `foo` module cannot be referenced by another module.

```python
# foo.er
x = "this is an invisible variable"
```

```python
#bar.er
foo = import "foo"
foo.x # AttributeError: Module 'foo' has no attribute 'x' ('x' is private)
```

On the other hand, there are also __public variables__, which can be referenced from the outside.
Public variables are defined with `.`.

```python
# foo.er
.x = "this is a visible variable"
```

```python
#bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

You don't need to add anything to private variables, but you can also add `::` or `self::` (`Self::` for types etc.) to indicate that they are private. increase. It can also be `module::` if it is a module.

```python
::x = "this is an invisible variable"
assert ::x == x
assert self ::x == ::x
assert module::x == ::x
```

In the context of purely sequential execution, private variables are almost synonymous with local variables. It can be referenced from the inner scope.

```python
::x = "this is a private variable"
y =
    x + 1 # exactly module::x
```

By using `::`, you can distinguish variables with the same name within the scope.
Specify the scope of the variable you want to refer to on the left. Specify `module` for the top level.
If not specified, the innermost variable is referenced as usual.

```python
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

In the anonymous subroutine scope, `self` specifies its own scope.

```python
x = 0
f = x ->
    log module::x, self::x
f1# 0 1
```

`::` is also responsible for accessing private instance attributes.

```python
x = 0
C = Class {x = Int}
C.
    # Top-level x is referenced (warning to use module::x)
    f1 self = x
    # instance attribute x is referenced
    f2 self = self::x
```

## Visibility in external modules

A class defined in one module can actually define 方法 from an external module.

```python
# foo.er
.Foo = Class()
```

```python
#bar.er
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

However, both of those 方法 are only available within that module.
Private 方法 defined externally are visible to 方法 of the `Foo` class only within the defining module.
Public 方法 are exposed outside the class, but not outside the module.

```python
# baz.er
{Foo; ...} = import "foo"

foo = Foo.new()
foo.public() # AttributeError: 'Foo' has no attribute 'public' ('public' is defined in module 'bar')
```

Also, 方法 cannot be defined in the type to be re-exported.
This is to avoid confusion about 方法 being found or not found depending on the module they are imported from.

```python
#bar.er
{.Foo; ...} = import "foo"

.Foo::
    private self = pass # Error
Foo.
    public self = self::private() # Error
```

If you want to do something like this, define a [patch](./type/07_patch.md).

```python
#bar.er
{Foo; ...} = import "foo"

FooImpl = Patch Foo
FooImpl :=:
    private self = pass
Foo Impl.
    public self = self::private()
```

```python
# baz.er
{Foo; ...} = import "foo"
{FooImpl; ...} = import "bar"

foo = Foo.new()
foo.public()
```

## restricted public variables

Variable visibility is not limited to complete public/private.
You can also publish with restrictions.

```python
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

```python
foo = import "foo"
_ = foo.record.a.x # VisibilityError
_ = foo.record.a.y # VisibilityError
_ = foo.record.a.z # OK
```

<p align='center'>
    <a href='./18_ownership.md'>Previous</a> | <a href='./20_naming_rule.md'>Next</a>
</p>