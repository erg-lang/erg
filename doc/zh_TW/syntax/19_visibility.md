# 可見性（Visibility）

Erg 中的變量具有的概念。我們看到的所有變量都稱為<gtr=“20”/>。這是一個外界不可見的變量。例如，在<gtr=“18”/>模塊中定義的私有變量不能被另一個模塊引用。


```erg
# foo.er
x = "this is an invisible variable"
```


```erg
# bar.er
foo = import "foo"
foo.x # AttributeError: Module 'foo' has no attribute 'x' ('x' is private)
```

與此相對，也有，這是可從外部參照的。公共變量定義為<gtr=“21”/>。


```erg
# foo.er
.x = "this is a visible variable"
```


```erg
# bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

你不需要為私有變量指定任何內容，但也可以指定或<gtr=“24”/>（例如，<gtr=“25”/>）以表明它是私有的。模塊也可以是<gtr=“26”/>。


```erg
::x = "this is a invisible variable"
assert ::x == x
assert self::x == ::x
assert module::x == ::x
```

在簡單的順序執行上下文中，私有變量幾乎等同於局部變量。從內部範圍可以參照。


```erg
::x = "this is a private variable"
y =
    x + 1 # 正確にはmodule::x
```

你可以使用來區分作用域中的同名變量。在左側指定要引用的變量的範圍。對於頂級，指定<gtr=“28”/>。如果未指定，則引用最內部的變量，就像在正常情況下一樣。


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

對於未命名子程序的範圍，指定其範圍。


```erg
x = 0
f = x ->
    log module::x, self::x
f 1 # 0 1
```

還負責訪問專用實例屬性。


```erg
x = 0
C = Class {x = Int}
C.
    # トップレベルのxが參照される(module::xにするようwarningが出る)
    f1 self = x
    # インスタンス屬性のxが參照される
    f2 self = self::x
```

## 外部模塊中的可見性

在模塊中定義的類實際上也可以從外部模塊定義方法。


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

但是，這兩種方法只能在模塊中使用。只有在定義模塊中，類的方法才能引用外部定義的私有方法。公開方法在類之外公開，但不在模塊之外公開。


```erg
# baz.er
{Foo; ...} = import "foo"

foo = Foo.new()
foo.public() # AttributeError: 'Foo' has no attribute 'public' ('public' is defined in module 'bar')
```

此外，不能為要 Re-export 的類型定義方法。這是為了防止導入模塊導致方法丟失或找到的混淆。


```erg
# bar.er
{.Foo; ...} = import "foo"

.Foo::
    private self = pass # Error
.Foo.
    public self = self::private() # Error
```

如果要這樣做，請定義。


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

## 受限制的公共變量

變量的可見性並不只有完全的公開或非公開。也可以有限制地發布。


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