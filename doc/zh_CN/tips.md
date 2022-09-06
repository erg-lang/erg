# 提示

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tips.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tips.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## 想要更改显示错误的语言

请为您的语言下载 Erg。
但是，外部库可能不支持多种语言。

## 只想更改记录的某些属性

```python
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; rest; ...} = record
mut_record = {.height = !height; ...rest}
```

## 想要隐藏变量

使用 Erg 无法在相同范围内进行遮蔽。 但是，如果范围发生变化，您可以重新定义它们(这是一种称为实例块的语法)。

````python
## 获取一个 T!-type 对象，最后将它作为 T 类型赋值给一个变量
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
````

## 想以某种方式重用最终类(不可继承的类)

您可以创建一个包装类。 这就是所谓的构图模式。

```python
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## 想使用不是字符串的枚举类型

可以定义其他语言中常见的传统枚举类型(代数数据类型)如下
如果您实现“单例”，则类和实例是相同的。
此外，如果您使用 `Enum`，则选择的类型会自动定义为重定向属性。

```python
Ok = Class Impl := Singleton
Err = Class Impl := Singleton
ErrWithInfo = Inherit {info = Str}
Status = Enum Ok, Err, ErrWithInfo
stat: Status = Status.cons(ErrWithInfo) {info = "error caused by ..."}
match! stat:
    Status.Ok -> ...
    Status.Err -> ...
    Status.ErrWithInfo::{info} -> ...
```

```python
Status = Enum Ok, Err, ErrWithInfo
# 相当于
Status = Class Ok or Err or ErrWithInfo
Status.
    Ok = Ok
    Err = Err
    ErrWithInfo = ErrWithInfo
```

## 我想在1开头枚举

方法一：

```python
arr = [...]
for! arr.iter().enumerate(start: 1), i =>
    ...
```

method 2:

```python
arr = [...]
for! arr.iter().zip(1...) , i =>
    ...
```

## 想要测试一个(白盒)非公共 API

`foo.er` 中的私有 API 可在 `foo.test.er` 模块中特别访问。
`foo.test.er` 模块无法导入，因此它保持隐藏状态。

```python
# foo.er
private x = ...
```

```python
# foo.test.er
foo = import "foo"

@Test
'testing private' x =
    ...
    y = foo::private x
    ...
```

## 想定义一个从外部只读的(变量)属性

您可以将属性设为私有并定义一个 getter。

```python
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## 希望在类型系统上识别参数名称

您可以按记录接收参数。

```python
Point = {x = Int; y = Int}

norm: Point -> Int
norm({x: Int; y: Int}): Int = x**2 + y**2
assert norm({x = 1; y = 2}) == norm({y = 2; x = 1})
```

## 想要停止警告

Erg 中没有停止警告的选项(这是设计使然)。 请重写你的代码。
