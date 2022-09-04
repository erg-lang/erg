# 装饰器（修饰符）

装饰器用于将特定的状态和行为添加到类型和函数中，或将其显式。装饰师的语法如下。


```erg
@deco
X = ...
```

装饰器可以有多个，除非冲突。

装饰器不是一个特殊的对象，它的实体只是一个参数函数。装饰器等效于以下伪代码。


```erg
X = ...
X = deco(X)
```

因为 Erg 不能重新赋值变量，所以上面的代码不能通过。对于简单的变量，这与相同，但对于即时块和子程序，这是不可能的，因此需要一个装饰器。


```erg
@deco
f x =
    y = ...
    x + y

# コードが横長になるのを防ぐこともできる
@LongNameDeco1
@LongNameDeco2
C = Class ...
```

下面介绍一些频出的嵌入式装饰器。

## Inheritable

指示所定义的类型是可继承类。如果将参数指定为<gtr=“10”/>，则外部模块类可以继承这些参数。默认为<gtr=“11”/>，不能从外部继承。

## Final

使方法不可覆盖。将类附加到类后，它将成为不可继承类，但这没有意义，因为这是缺省类。

## Override

用于覆盖属性。缺省情况下，Erg 会在尝试定义与基类相同的属性时出错。

## Impl

指示要实现自变量的特写。


```erg
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

## Attach

指定默认情况下随托盘一起提供的附件曲面片。这样，你就可以重现与 Rust 的trait相同的行为。


```erg
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

这将在从其他模块导入托盘时自动应用附件修补程序。


```erg
# 本来IntIsBinAdd, OddIsBinAddも同時にインポートする必要があるが、アタッチメントパッチなら省略可
{BinAdd; ...} = import "foo"

assert Int.AddO == Int
assert Odd.AddO == Even
```

在内部，我们只是使用trait的方法将其连接起来。如果发生冲突，可以使用trait的<gtr=“13”/>方法将其移除。


```erg
@Attach X
T = Trait ...
assert X in T.attaches
U = T.detach(X).attach(Y)
assert X not in U.attaches
assert Y in U.attaches
```

## Deprecated

表示变量规范已过时。

## Test

指示测试子程序。测试子例程使用命令执行。

<p align='center'>
    <a href='./28_spread_syntax.md'>Previous</a> | <a href='./30_error_handling.md'>Next</a>
</p>
