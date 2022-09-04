# 可視性(Visibility)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/19_visibility.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/19_visibility.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

Ergの変数には __可視性__ という概念が存在します。
今まで見てきた変数は全て __プライベート変数(非公開変数)__ と呼ばれます。これは、外部から不可視の変数です。
例えば`foo`モジュールで定義したプライベート変数は、別のモジュールから参照できないのです。

```python
# foo.er
x = "this is an invisible variable"
```

```python
# bar.er
foo = import "foo"
foo.x # AttributeError: Module 'foo' has no attribute 'x' ('x' is private)
```

対して、 __パブリック(公開)変数__ というものもあり、こちらは外部から参照できます。
公開変数は`.`を付けて定義します。

```python
# foo.er
.x = "this is a visible variable"
```

```python
# bar.er
foo = import "foo"
assert foo.x == "this is a visible variable"
```

非公開変数には何も付ける必要はないのですが、非公開であることを明示するために`::`または`self::`(型などなら`Self::`)を付けることもできます。またモジュールなら`module::`とすることもできます。

```python
::x = "this is a invisible variable"
assert ::x == x
assert self::x == ::x
assert module::x == ::x
```

単なる逐次実行の文脈では、プライベート変数はローカル変数とほぼ同義です。内側のスコープからは参照することが出来ます。

```python
::x = "this is a private variable"
y =
    x + 1 # 正確にはmodule::x
```

`::`を使うことで、スコープ内の同名変数の区別ができます。
参照したい変数のスコープを左側に指定します。トップレベルの場合は`module`を指定します。
指定しなかった場合は通常の場合と同じく最も内側の変数が参照されます。

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

無名サブルーチンのスコープでは`self`で自身のスコープを指定します。

```python
x = 0
f = x ->
    log module::x, self::x
f 1 # 0 1
```

`::`は、プライベートインスタンス属性にアクセスするという役割も持っています。

```python
x = 0
C = Class {x = Int}
C.
    # トップレベルのxが参照される(module::xにするようwarningが出る)
    f1 self = x
    # インスタンス属性のxが参照される
    f2 self = self::x
```

## 外部モジュールでの可視性

あるモジュールで定義されたクラスは、実は外部モジュールからでもメソッドを定義できます。

```python
# foo.er
.Foo = Class()
```

```python
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

ただし、そのメソッドを使えるのはどちらもそのモジュール内でのみです。
外部で定義された非公開メソッドは、定義モジュール内でのみ`Foo`クラスのメソッドから参照できます。
公開メソッドはクラスの外には公開されますが、モジュール外までは公開されません。

```python
# baz.er
{Foo; ...} = import "foo"

foo = Foo.new()
foo.public() # AttributeError: 'Foo' has no attribute 'public' ('public' is defined in module 'bar')
```

また、Re-exportする型にメソッドを定義することはできません。
インポート元のモジュールによってメソッドが見つかったり見つからなかったりといった混乱を防ぐためです。

```python
# bar.er
{.Foo; ...} = import "foo"

.Foo::
    private self = pass # Error
.Foo.
    public self = self::private() # Error
```

このようなことを行いたい場合は[パッチ](./type/07_patch.md)を定義します。

```python
# bar.er
{Foo; ...} = import "foo"

FooImpl = Patch Foo
FooImpl :=:
    private self = pass
FooImpl.
    public self = self::private()
```

```python
# baz.er
{Foo; ...} = import "foo"
{FooImpl; ...} = import "bar"

foo = Foo.new()
foo.public()
```

## 制限公開変数

変数の可視性は完全な公開・非公開しかないわけではありません。
制限付きで公開することもできます。

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
