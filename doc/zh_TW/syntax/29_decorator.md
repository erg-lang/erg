# 装饰器(修饰符)

装饰器用于向类型或函数添加或演示特定状态或行为。
装饰器的语法如下。

```python
@deco
X=...
```

你可以有多个装饰器，只要它们不冲突。

装饰器不是一个特殊的对象，它只是一个单参数函数。 装饰器等价于下面的伪代码。

```python
X=...
X = deco(X)
```

Erg 不允许重新分配变量，因此上面的代码不起作用。
对于简单的变量，它与`X = deco(...)` 相同，但对于即时块和子例程，你不能这样做，所以你需要一个装饰器。

```python
@deco
f x =
    y = ...
    x + y

# 还可以防止代码变成水平的
@LongNameDeco1
@LongNameDeco2
C = Class...
```

下面是一些常用的内置装饰器。

## 可继承

指示定义类型是可继承的类。 如果为参数 `scope` 指定 `"public"`，甚至可以继承外部模块的类。 默认情况下它是`"private"`，不能被外部继承。

＃＃ 最后

使该方法不可覆盖。 将它添加到类中使其成为不可继承的类，但由于它是默认值，因此没有意义。

## 覆盖

覆盖属性时使用。 默认情况下，如果您尝试定义与基类相同的属性，Erg 将抛出错误。

## 实现

表示参数 trait 已实现。

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

## 附

指定默认情况下随 trait 附带的附件补丁。
这允许您重现与 Rust 特征相同的行为。

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

当从其他模块导入特征时，这将自动应用附件补丁。

```Python
# 本来应该同时导入IntIsBinAdd和OddIsBinAdd，但是如果是附件补丁可以省略
{BinAdd; ...} = import "foo"

assert Int. AddO == Int
assert Odd.AddO == Even
```

在内部，它只是使用 trait 的 .attach 方法附加的。 可以使用 trait 的 `.detach` 方法消除冲突。

```python
@Attach X
T = Trait...
assert X in T. attaches
U = T.detach(X).attach(Y)
assert X not in U. attaches
assert Y in U. attaches
```

## 已弃用

指示变量规范已过时且不推荐使用。

＃＃ 测试

表示这是一个测试子例程。 测试子程序使用 `erg test` 命令运行。

<p align='center'>
    <a href='./28_spread_syntax.md'>上一页</a> | <a href='./30_error_handling.md'>下一页</a>
</p>