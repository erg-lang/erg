# 可变类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/18_mut.md%26commit_hash%3D00682a94603fed2b531898200a79f2b4a64d5aae)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/18_mut.md&commit_hash=00682a94603fed2b531898200a79f2b4a64d5aae)

> __Warning__: 本节中的信息是旧的并且包含一些错误

默认情况下，Erg 中的所有类型都是不可变的，即它们的内部状态无法更新
但是你当然也可以定义可变类型。变量类型用 `!` 声明

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is {self::name}. I am {self::age}."
    inc_age!ref!self = self::name.update!old -> old + 1
```

准确地说，基类型是可变类型或包含可变类型的复合类型的类型必须在类型名称的末尾有一个"！"。没有 `!` 的类型可以存在于同一个命名空间中，并被视为单独的类型
在上面的例子中，`.age` 属性是可变的，`.name` 属性是不可变的。如果即使一个属性是可变的，那么整个属性也是可变的

可变类型可以定义重写实例的过程方法，但具有过程方法并不一定使它们可变。例如数组类型`[T; N]` 实现了一个 `sample!` 随机选择一个元素的方法，但当然不会破坏性地修改数组

对可变对象的破坏性操作主要是通过 .update! 方法完成的。`.update!` 方法是一个高阶过程，它通过应用函数 `f` 来更新 `self`

```python
i = !1
i.update! old -> old + 1
assert i == 2
```

`.set!` 方法只是丢弃旧内容并用新值替换它。.set!x = .update!_ -> x

```python
i = !1
i.set! 2
assert i == 2
```

`.freeze_map` 方法对不变的值进行操作

```python
a = [1, 2, 3].into [Nat; !3]
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(Array)
```

在多态不可变类型中，该类型的类型参数"T"被隐式假定为不可变

```python
# ImmutType < Type
KT: ImmutType = Class ...
K!T: Type = Class ...
```

在标准库中，变量 `(...)!` 类型通常基于不可变 `(...)` 类型。但是，`T!` 和 `T` 类型没有特殊的语言关系，并且不能这样构造 [<sup id="f1">1</sup>](#1) 

请注意，有几种类型的对象可变性
下面我们将回顾内置集合类型的不可变/可变语义

```python
# 数组类型
## 不可变类型
[T; N] # 不能执行可变操作
## 可变类型
[T; N] # 可以一一改变内容
[T; !N] # 可变长度，内容不可变但可以通过添加/删除元素来修改
[!T; N] # 内容是不可变的对象，但是可以替换成不同的类型(实际上可以通过不改变类型来替换)
[!T; !N] # 类型和长度可以改变
[T; !N] # 内容和长度可以改变
[!T!; N] # 内容和类型可以改变
[!T!; !N] # 可以执行各种可变操作
```

当然，您不必全部记住和使用它们
对于可变数组类型，只需将 `!` 添加到您想要可变的部分，实际上是 `[T; N]`, `[T!; N]`，`[T; !N]`, ` [T!; !N]` 可以涵盖大多数情况

这些数组类型是语法糖，实际类型是: 

```python
# actually 4 types
[T; N] = Array(T, N)
[T; !N] = Array!(T, !N)
[!T; N] = ArrayWithMutType!(!T, N)
[!T; !N] = ArrayWithMutTypeAndLength!(!T, !N)
[T!; !N] = Array!(T!, !N)
[!T!; N] = ArrayWithMutType!(!T!, N)
[!T!; !N] = ArrayWithMutTypeAndLength!(!T!, !N)
```

这就是能够改变类型的意思

```python
a = [1, 2, 3].into [!Nat; 3]
a.map!(_ -> "a")
a: [!Str; 3]
```

其他集合类型也是如此

```python
# 元组类型
## 不可变类型
(T, U) # 元素个数不变，内容不能变
## 可变类型
(T!, U) # 元素个数不变，第一个元素可以改变
(T，U)！ # 元素个数不变，内容可以替换
...
```

```python
# 设置类型
## 不可变类型
{T; N} # 不可变元素个数，内容不能改变
## 可变类型
{T！; N} # 不可变元素个数，内容可以改变(一个一个)
{T; N}！ # 可变元素个数，内容不能改变
{T！; N}！ # 可变元素个数，内容可以改变
...
```

```python
# 字典类型
## 不可变类型
{K: V} # 长度不可变，内容不能改变
## 可变类型
{K:V!} # 恒定长度，值可以改变(一一)
{K: V}！ # 可变长度，内容不能改变，但可以通过添加或删除元素来增加或删除，内容类型也可以改变
...
```

```python
# 记录类型
## 不可变类型
{x = Int; y = Str} # 内容不能改变
## 可变类型
{x = Int！; y = Str} # 可以改变x的值
{x = Int; y = Str}！ # 替换 {x = Int; 的任何实例 y = Str}
...
```

一个类型 `(...)` 简单地变成了 `T! = (...)!` 当 `T = (...)` 被称为简单结构化类型。简单的结构化类型也可以(语义上)说是没有内部结构的类型
数组、元组、集合、字典和记录类型都是非简单的结构化类型，但 Int 和 Sieve 类型是

```python
# 筛子类型
## 枚举
{1, 2, 3} # 1, 2, 3 之一，不可更改
{1、2、3}！ # 1、2、3，可以改
## 区间类型
1..12 # 1到12，不能改
1..12！ # 1-12中的任意一个，你可以改变
## 筛型(普通型)
{I: Int | I % 2 == 0} # 偶数类型，不可变
{I: Int | I % 2 == 0} # 偶数类型，可以改变
{I: Int | I % 2 == 0}！ # 与上面完全相同的类型，但上面的表示法是首选
```

从上面的解释来看，可变类型不仅包括自身可变的，还包括内部类型可变的
诸如 `{x: Int!}` 和 `[Int!; 之类的类型3]` 是内部可变类型，其中内部的对象是可变的，而实例本身是不可变的

对于具有内部结构并在类型构造函数本身上具有 `!` 的类型 `K!(T, U)`，`*self` 可以更改整个对象。也可以进行局部更改
但是，希望尽可能保持本地更改权限，因此如果只能更改 `T`，最好使用 `K(T!, U)`
而对于没有内部结构的类型‘T!’，这个实例只是一个可以交换的‘T’盒子。方法不能更改类型

---

<span id="1" style="font-size:x-small"><sup>1</sup> `T!` 和 `T` 类型没有特殊的语言关系是有意的。这是一个设计。如果存在关系，例如命名空间中存在`T`/`T!`类型，则无法从其他模块引入`T!`/`T`类型。此外，可变类型不是为不可变类型唯一定义的。给定定义 `T = (U, V)`，`T!` 的可能变量子类型是 `(U!, V)` 和 `(U, V!)`。[↩](#f1)</span>

<p align='center'>
    <a href='./17'>上一页</a> | <a href='./19_bound.md'>下一页</a>
</p>