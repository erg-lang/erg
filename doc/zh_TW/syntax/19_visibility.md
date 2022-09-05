# 可見性

Erg 變量具有 __visibility__ 的概念。
到目前為止，我們看到的所有變量都稱為 __private variables__。 這是一個外部不可見的變量。
例如，`foo` 模塊中定義的私有變量不能被另一個模塊引用。

```python
# foo.er
x = "this is an invisible variable"
```

```python
#bar.er
foo = import "foo"
foo.x # AttributeError: 模塊 'foo' 沒有屬性 'x' ('x' 是私有的)
```

另一方面，也有__public variables__，可以從外部引用。
公共變量用`.`定義。

```python
# foo.er
.x = "this is a visible variable"
```

```python
#bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

您不需要向私有變量添加任何內容，但您也可以添加 `::` 或 `self::`(用于類型等的`Self::`)以表明它們是私有的。 增加。 如果它是一個模塊，它也可以是 `module::`

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
    x + 1 # 完全是 module::x
```

通過使用`::`，可以區分作用域內同名的變量。
在左側指定要引用的變量的范圍。 為頂層指定 `module`。
如果未指定，則照常引用最里面的變量。

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

在匿名子程序作用域中，`self` 指定了它自己的作用域

```python
x = 0
f = x ->
    log module::x, self::x
f1# 0 1
```

`::` 還負責訪問私有實例屬性。

```python
x = 0
C = Class {x = Int}
C.
    # 頂級 x 被引用(警告使用 module::x)
    f1 self = x
    # 實例屬性 x 被引用
    f2 self = self::x
```

## 外部模塊中的可見性

在一個模塊中定義的類實際上可以定義來自外部模塊的方法。

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
    foo::private() # 屬性錯誤
```

但是，這兩種方法都只在該模塊中可用。
外部定義的私有方法對 Foo 類的方法僅在定義模塊內可見。
公共方法暴露在類之外，但不在模塊之外。

```python
# baz.er
{Foo; ...} = import "foo"

foo = Foo.new()
foo.public() # 屬性錯誤：“Foo”沒有屬性“public”(“public”在模塊“bar”中定義)
```

此外，方法不能在要重新導出的類型中定義。
這是為了避免混淆方法是否找到，具體取決于導入方法的模塊。

```python
#bar.er
{.Foo; ...} = import "foo"

.Foo::
    private self = pass # 錯誤
Foo.
    public self = self::private() # 錯誤
```

如果你想做這樣的事情，定義一個 [patch](./type/07_patch.md)。

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

## 受限公共變量

可變可見性不限于完全公共/私有。
您也可以有限制地發布。

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

_ = .record.a.x # 可見性錯誤
_ = .record.a.y # OK
_ = .record.a.z # OK
```

```python
foo = import "foo"
_ = foo.record.a.x # 可見性錯誤
_ = foo.record.a.y # 可見性錯誤
_ = foo.record.a.z # OK
```

<p align='center'>
    <a href='./18_ownership.md'>上一頁</a> | <a href='./20_naming_rule.md'>下一頁</a>
</p>