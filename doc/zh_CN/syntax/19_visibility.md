# 可见性（Visibility）

Erg 中的变量具有的概念。我们看到的所有变量都称为<gtr=“20”/>。这是一个外界不可见的变量。例如，在<gtr=“18”/>模块中定义的私有变量不能被另一个模块引用。


```erg
# foo.er
x = "this is an invisible variable"
```


```erg
# bar.er
foo = import "foo"
foo.x # AttributeError: Module 'foo' has no attribute 'x' ('x' is private)
```

与此相对，也有，这是可从外部参照的。公共变量定义为<gtr=“21”/>。


```erg
# foo.er
.x = "this is a visible variable"
```


```erg
# bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

你不需要为私有变量指定任何内容，但也可以指定或<gtr=“24”/>（例如，<gtr=“25”/>）以表明它是私有的。模块也可以是<gtr=“26”/>。


```erg
::x = "this is a invisible variable"
assert ::x == x
assert self::x == ::x
assert module::x == ::x
```

在简单的顺序执行上下文中，私有变量几乎等同于局部变量。从内部范围可以参照。


```erg
::x = "this is a private variable"
y =
    x + 1 # 正確にはmodule::x
```

你可以使用来区分作用域中的同名变量。在左侧指定要引用的变量的范围。对于顶级，指定<gtr=“28”/>。如果未指定，则引用最内部的变量，就像在正常情况下一样。


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

对于未命名子程序的范围，指定其范围。


```erg
x = 0
f = x ->
    log module::x, self::x
f 1 # 0 1
```

还负责访问专用实例属性。


```erg
x = 0
C = Class {x = Int}
C.
    # トップレベルのxが参照される(module::xにするようwarningが出る)
    f1 self = x
    # インスタンス属性のxが参照される
    f2 self = self::x
```

## 外部模块中的可见性

在模块中定义的类实际上也可以从外部模块定义方法。


```erg
# foo.er
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

但是，这两种方法只能在模块中使用。只有在定义模块中，类的方法才能引用外部定义的私有方法。公开方法在类之外公开，但不在模块之外公开。


```erg
# baz.er
{Foo; ...} = import "foo"

foo = Foo.new()
foo.public() # AttributeError: 'Foo' has no attribute 'public' ('public' is defined in module 'bar')
```

此外，不能为要 Re-export 的类型定义方法。这是为了防止导入模块导致方法丢失或找到的混淆。


```erg
# bar.er
{.Foo; ...} = import "foo"

.Foo::
    private self = pass # Error
.Foo.
    public self = self::private() # Error
```

如果要这样做，请定义。


```erg
# bar.er
{Foo; ...} = import "foo"

FooImpl = Patch Foo
FooImpl :=:
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

## 受限制的公共变量

变量的可见性并不只有完全的公开或非公开。也可以有限制地发布。


```erg
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


```erg
foo = import "foo"
_ = foo.record.a.x # VisibilityError
_ = foo.record.a.y # VisibilityError
_ = foo.record.a.z # OK
```

<p align='center'>
    <a href='./18_ownership.md'>Previous</a> | <a href='./20_naming_rule.md'>Next</a>
</p>
