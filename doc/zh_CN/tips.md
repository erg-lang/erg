# Tips

## 我想改变错误的显示语言

请下载语言版本的 erg。但是，标准库之外可能不提供多语言支持。

## 只想改变记录的特定属性


```erg
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; rest; ...} = record
mut_record = {.height = !height; ...rest}
```

## 我要阴影变量

Erg 不能在同一范围内进行阴影。但是，如果作用域变了，就可以重新定义，所以最好使用即时块。


```erg
# T!型オブジェクトを取得し、最終的にT型として変数へ代入
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
```

## 想办法重用 final class（不可继承类）

我们来做个说唱班。这就是所谓的合成模式。


```erg
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## 要使用非字符串枚举类型

你可以定义其他语言中常见的传统枚举类型（代数数据类型），如下所示。当实现时，类和实例是等同的。此外，如果使用<gtr=“13”/>，则可选择的类型将自动定义为重定向属性。


```erg
Ok = Class Impl := Singleton
Err = Class Impl := Singleton
ErrWithInfo = Inherit {info = Str}
Status = Enum Ok, Err, ErrWithInfo
stat: Status = Status.cons(ErrWithInfo) {info = "error caused by ..."}
match! stat:
    Status.Ok -> ...
    Status.Err -> ...
    Status.ErrWithInfo::{info;} -> ...
```


```erg
Status = Enum Ok, Err, ErrWithInfo
# is equivalent to
Status = Class Ok or Err or ErrWithInfo
Status.
    Ok = Ok
    Err = Err
    ErrWithInfo = ErrWithInfo
```

## 一开始想要 enumerate

method 1:


```erg
arr = [...]
for! arr.iter().enumerate(start: 1), i =>
    ...
```

method 2:


```erg
arr = [...]
for! arr.iter().zip(1..), i =>
    ...
```

## 我想测试我的私有 API（白盒）

名为的模块可以专门访问<gtr=“15”/>的专用 API。<gtr=“16”/>模块不能导入，因此保持了隐藏性。


```erg
# foo.er
private x = ...
```


```erg
# foo.test.er
foo = import "foo"

@Test
'testing private' x =
    ...
    y = foo::private x
    ...
```

## 要定义外部只读（可变）属性

最好将属性设为私有，然后定义 getta。


```erg
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## 要在类型系统上标识参数名称

将参数作为记录接收比较好。


```erg
Point = {x = Int; y = Int}

norm: Point -> Int
norm({x: Int; y: Int}): Int = x**2 + y**2
assert norm({x = 1; y = 2}) == norm({y = 2; x = 1})
```

## 我不想发出警告

没有用于阻止 Erg 警告的选项（这是故意的设计）。重写代码。
