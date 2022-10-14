# 解决补丁方法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/trait_method_resolving.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/trait_method_resolving.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

`Nat` 是零个或多个`Int`，`Int` 的子类型
`Nat` 在 Python 类层次结构中不存在。我想知道 Erg 是如何解决这个补丁方法的?

```python
1.times do:
    log "hello world"
```

`.times` 是一种 `NatImpl` 补丁方法
由于`1`是`Int`的一个实例，首先通过跟踪`Int`的MRO(方法解析顺序)来搜索它
Erg 在 `Int` 的 MRO 中有 `Int`、`Object`。它来自 Python(Python 中的`int.__mro__ == [int, object]`)
`.times` 方法在它们中都不存在。现在让我们探索那个子类型

~

整数显然应该在其超类型中包含实数、复数甚至整数，但这一事实并没有出现在 Python 兼容层中
然而，`1 in Complex` 和 `1 in Num` 在 Erg 中实际上是 `True`
至于`Complex`，即使是与`Int`没有继承关系的类，也被判断为类型兼容。这到底是怎么回事?

~

一个对象有无数种它所属的类型
但是我们真的只需要考虑带有方法的类型，即带有名称的类型

Erg 编译器有一个补丁类型的哈希图，其中包含所有提供的方法及其实现
每次定义新类型时都会更新此表

```python
provided_method_table = {
    ...
    "foo": [Foo],
    ...
    ".times": [Nat, Foo],
    ...
}
```

具有 `.times` 方法的类型是 `Nat`、`Foo`。从这些中，找到与"{1}"类型匹配的一个
有两种类型的符合性确定。它们是筛式判断和记录式判断。这是通过筛子类型确定来完成的

##筛型确定

检查候选类型是否与 `1` 的类型 `{1}` 兼容。与"{1}"兼容的筛子类型有"{0, 1}"、"0..9"等
有限元代数类型，例如 `0..1 或 3..4`、`-1..2 和 0..3`，在声明为基本类型(即 {0, 1, 3, 4}`，`{0, 1, 2}`)
在这种情况下，`Nat` 是 `0.._ == {I: Int | I >= 0}`，所以 `{1}` 与 `Nat` 兼容

## 确定记录类型

检查候选类型是否与 `Int` 兼容，1 类
其他是`Int`的修复程序并且`Int`具有所有必需属性的也是兼容的

~

所以`Nat`适合。但是，如果 `Foo` 也匹配，则由 `Nat` 和 `Foo` 之间的包含关系决定
即，选择子类型方法
如果两者之间没有包含关系，则会发生编译错误(这是一种安全措施，防止违背程序员的意图执行方法)
要消除错误，您需要明确指定补丁

```python
o.method(x) -> P.method(o, x)
```

## 通用方法解析修补程序

像这样定义一个补丁: 

```python
FnType T: Type = Patch T -> T
FnType.type = T
```

在 `FnType` 补丁下可以使用如下代码。我想知道这将如何解决

```python
assert (Int -> Int).type == Int
```

首先，`FnType(T)` 以下列格式注册到`provided_method_table` 中

```python
provided_method_table = {
    ...
    "type": [FnType(T)],
    ...
}
```

`FnType(T)` 检查匹配类型。在这种情况下，`FnType(T)` 补丁类型是 `Type -> Type`
这匹配 `Int -> Int`。如果合适，进行单态化和替换(取 `T -> T` 和 `Int -> Int`、`{T => Int}` 的差异)

```python
assert FnType(Int).type == Int
```