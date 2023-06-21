# 可視性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/21_visibility.md%26commit_hash%3D5fe4ad12075d710910f75c40552b4db621904c57)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/21_visibility.md&commit_hash=5fe4ad12075d710910f75c40552b4db621904c57)

Ergの変数には __可視性__ という概念が存在します。
今まで見てきた変数は全て __プライベート変数(非公開変数)__ と呼ばれます。これは、外部から不可視の変数です。
例えば`foo`モジュールで定義したプライベート変数は、別のモジュールから参照できないのです。

```python
# foo.er
x = "this is an invisible variable"
```

```python,compile_fail
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

```python,checker_ignore
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
.Bar = Class()
```

```python
# bar.er
{Foo;} = import "foo"

Foo::
    private self = pass
Foo.
    public self = self::private()
```

```python,compile_fail
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
{Foo;} = import "foo"

foo = Foo.new()
foo.public() # AttributeError: 'Foo' has no attribute 'public' ('public' is defined in module 'bar')
```

また、Re-exportする型にメソッドを定義することはできません。
インポート元のモジュールによってメソッドが見つかったり見つからなかったりといった混乱を防ぐためです。

```python,compile_fail
# bar.er
{.Foo;} = import "foo"

.Foo::
    private self = pass # Error
.Foo.
    public self = self::private() # Error
```

このようなことを行いたい場合は[パッチ](./type/07_patch.md)を定義します。

```python
# bar.er
{Foo;} = import "foo"

FooImpl = Patch Foo
FooImpl :=:
    private self = pass
FooImpl.
    public self = self::private()
```

```python
# baz.er
{Foo;} = import "foo"
{FooImpl;} = import "bar"

foo = Foo.new()
foo.public()
```

## 制限公開変数

変数の可視性は完全な公開・非公開しかないわけではありません。
制限付きで公開することもできます。

`.`の後に`[]`を付け、その中に「公開する最大の名前空間[<sup id="f1">1</sup>](#1)の識別子」を指定します。
下の例では、`.[.record]`は`.record`の名前空間内でのみ、`.[module]`はモジュール内でのみ公開されます。

```python,checker_ignore
# foo.er
.record = {
    .a = {
        .[.record]x = 0
        .[module]y = 0
        .z = 0
    }
    _ = .a.x # OK
    _ = .a.y # OK
    _ = .a.z # OK
}

func x =
    _ = .record.a.x # VisibilityError
    _ = .record.a.y # OK
    _ = .record.a.z # OK
    None

_ = .record.a.x # VisibilityError
_ = .record.a.y # OK
_ = .record.a.z # OK
```

```python,checker_ignore
foo = import "foo"
_ = foo.record.a.x # VisibilityError
_ = foo.record.a.y # VisibilityError
_ = foo.record.a.z # OK
```

名前空間はコンマ区切りで複数指定することも出来ます。

```python,checker_ignore
.[.record, func]x = 0
```

ところで、クラスのプライベート属性はサブクラスからアクセス出来ません。

```python,compile_fail
C = Class {i = Int}

D = Inherit C
D.
    f self = self::i # VisibilityError
```

あるサブクラスからアクセスできるようにしたい場合は、以下のように指定します。

```python
C = Class {.[D]i = Int}

D = Inherit C
D.
    f self = self.i
```

サブクラス全体に公開する場合は、`.[<: Self]`とします。
これは他言語では`protected`に相当するものです。

```python
C = Class {.[<: C]i = Int}
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> Ergにおいて名前空間は、名前とオブジェクトの対応の集合を指す。インスタントスコープを作る変数の識別子やモジュール・関数・クラス・レコードが名前空間と同一視される。関数・クラス・レコードは識別子に束縛せずに生成することができるため、これらは本来無名名前空間を作る。しかし識別子に束縛されると、識別子と同名の名前で上書きされる。[↩](#f1) </span>

<p align='center'>
    <a href='./20_ownership.md'>Previous</a> | <a href='./22_naming_rule.md'>Next</a>
</p>
