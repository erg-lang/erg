# 可见性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/19_visibility.md%26commit_hash%3D20aa4f02b994343ab9600317cebafa2b20676467)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/19_visibility.md&commit_hash=20aa4f02b994343ab9600317cebafa2b20676467)

Erg 变量具有 __visibility__ 的概念
到目前为止，我们看到的所有变量都称为 __private variables__。这是一个外部不可见的变量
例如，`foo` 模块中定义的私有变量不能被另一个模块引用

```python
# foo.er
x = "this is an invisible variable"
```

```python,compile_fail
# bar.er
foo = import "foo"
foo.x # AttributeError: 模块 'foo' 没有属性 'x' ('x' 是私有的)
```

另一方面，也有__public variables__，可以从外部引用
公共变量用`.`定义

```python
# foo.er
.x = "this is a visible variable"
```

```python
# bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

您不需要向私有变量添加任何内容，但您也可以添加 `::` 或 `self::`(用于类型等的`Self::`)以表明它们是私有的。增加。如果它是一个模块，它也可以是 `module::`

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

通过使用`::`，可以区分作用域内同名的变量
在左侧指定要引用的变量的范围。为顶层指定 `module`
如果未指定，则照常引用最里面的变量

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

`::` 还负责访问私有实例属性

```python
x = 0
C = Class {x = Int}
C.
    # 顶级 x 被引用(警告使用 module::x)
    f1 self = x
    # 实例属性 x 被引用
    f2 self = self::x
```

## 外部模块中的可见性

在一个模块中定义的类实际上可以定义来自外部模块的方法

```python,compile_fail
# foo.er
.Foo = Class()
```

```python
# bar.er
{Foo;} = import "foo"

Foo::
    private self = pass
Foo.
    public self = self::private()

.f() =
    foo = Foo.new()
    foo.public()
    foo::private() # 属性错误
```

但是，这两种方法都只在该模块中可用
外部定义的私有方法对 Foo 类的方法仅在定义模块内可见
公共方法暴露在类之外，但不在模块之外

```python
# baz.er
{Foo;} = import "foo"

foo = Foo.new()
foo.public() # 属性错误: "Foo"没有属性"public"("public"在模块"bar"中定义)
```

此外，方法不能在要重新导出的类型中定义
这是为了避免混淆方法是否找到，具体取决于导入方法的模块

```python,compile_fail
# bar.er
{.Foo;} = import "foo"

.Foo::
    private self = pass # 错误
Foo.
    public self = self::private() # 错误
```

如果你想做这样的事情，定义一个 [patch](./type/07_patch.md)

```python
# bar.er
{Foo;} = import "foo"

FooImpl = Patch Foo
FooImpl :=:
    private self = pass
Foo Impl.
    public self = self::private()
```

```python
# baz.er
{Foo;} = import "foo"
{FooImpl;} = import "bar"

foo = Foo.new()
foo.public()
```

## 受限公共变量

可变可见性不限于完全公共/私有
您也可以有限制地发布

```python,checker_ignore
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

_ = .record.a.x # 可见性错误
_ = .record.a.y # OK
_ = .record.a.z # OK
```

```python,checker_ignore
foo = import "foo"
_ = foo.record.a.x # 可见性错误
_ = foo.record.a.y # 可见性错误
_ = foo.record.a.z # OK
```

<p align='center'>
    <a href='./19_ownership.md'>上一页</a> | <a href='./21_naming_rule.md'>下一页</a>
</p>