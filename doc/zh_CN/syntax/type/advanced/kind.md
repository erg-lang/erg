# Kind

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/kind.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/kind.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

一切都在 Erg 中输入。类型本身也不例外。__kind__ 表示"类型的类型"。例如，`Int` 属于 `Type`，就像 `1` 属于 `Int`。`Type` 是最简单的一种，__atomic kind__。在类型论符号中，`Type` 对应于 `*`

在Kind的概念中，实际上重要的是一种或多种Kind(多项式Kind)。单项类型，例如`Option`，属于它。一元Kind表示为 `Type -> Type` [<sup id="f1">1</sup>](#1)。诸如 `List` 或 `Option` 之类的 __container__ 特别是一种以类型作为参数的多项式类型
正如符号 `Type -> Type` 所表明的，`Option` 实际上是一个接收类型 `T` 并返回类型 `Option T` 的函数。但是，由于这个函数不是通常意义上的函数，所以通常称为一元类

注意`->`本身，它是一个匿名函数操作符，当它接收一个类型并返回一个类型时，也可以看作是一Kind型

另请注意，不是原子Kind的Kind不是类型。正如 `-1` 是一个数字但 `-` 不是，`Option Int` 是一个类型但 `Option` 不是。`Option` 等有时被称为类型构造函数

```python
assert not Option in Type
assert Option in Type -> Type
```

所以像下面这样的代码会报错:
在 Erg 中，方法只能在原子类型中定义，并且名称 `self` 不能在方法的第一个参数以外的任何地方使用

```python
# K 是一元类型
K: Type -> Type
K T = Class...
K.
foo x = ... # OK，这就像是所谓的静态方法
     bar self, x = ... # 类型错误: 无法为非类型对象定义方法
K(T).
    baz self, x = ... # OK
```

二进制或更高类型的示例是 `{T: U}`(: `(Type, Type) -> Type`), `(T, U, V)`(: `(Type, Type, Type) - > Type`), ... 等等

还有一个零项类型`() -> Type`。这有时等同于类型论中的原子类型，但在 Erg 中有所区别。一个例子是`类`

```python
Nil = Class()
```

## 收容类

多项类型之间也存在部分类型关系，或者更确切地说是部分类型关系

```python
K T = ...
L = Inherit K
L<: K
```

也就是说，对于任何 `T`，如果 `L T <: K T`，则 `L <: K`，反之亦然

```python
∀T. L T <: K T <=> L <: K
```

## 高阶Kind

还有一种高阶Kind。这是一种与高阶函数相同的概念，一种自身接收一种类型。`(Type -> Type) -> Type` 是一种更高的Kind。让我们定义一个属于更高Kind的对象

```python
IntContainerOf K: Type -> Type = K Int
assert IntContainerOf Option == Option Int
assert IntContainerOf Result == Result Int
assert IntContainerOf in (Type -> Type) -> Type
```

多项式类型的有界变量通常表示为 K, L, ...，其中 K 是 Kind 的 K

## 设置Kind

在类型论中，有记录的概念。这与 Erg 记录 [<sup id="f2">2</sup>](#2) 几乎相同

```python
# 这是一条记录，对应于类型论中所谓的记录
{x = 1; y = 2}
```

当所有的记录值都是类型时，它是一种类型，称为记录类型

```python
assert {x = 1; y = 2} in {x = Int; y = Int}
```

记录类型键入记录。一个好的猜测者可能认为应该有一个"记录类型"来键入记录类型。实际上它是存在的

```python
log Typeof {x = Int; y = Int} # {{x = Int; y = Int}}
```

像 `{{x = Int; 这样的类型 y = Int}}` 是一种记录类型。这不是一个特殊的符号。它只是一个枚举类型，只有 `{x = Int; y = Int}` 作为一个元素

```python
Point = {x = Int; y = Int}
Pointy = {Point}
```

记录类型的一个重要属性是，如果 `T: |T|` 和 `U <: T` 则 `U: |T|`
从枚举实际上是筛子类型的语法糖这一事实也可以看出这一点
枚举实际上是细化类型的语法糖，这一点也很明显。

```python
# {c} == {X: T | X == c} 对于普通对象，但是不能为类型定义相等性，所以 |T| == {X | X <: T}
{Point} == {P | P <: Point}
```

类型约束中的 `U <: T` 实际上是 `U: |T|` 的语法糖
作为此类类型的集合的种类通常称为集合种类。Setkind 也出现在迭代器模式中

```python
Iterable T = Trait {
    .Iterator = {Iterator}
    .iter = (self: Self) -> Self.Iterator T
}
```

## 多项式类型的类型推断

```python
Container K: Type -> Type, T: Type = Patch K(T, T)
Container (K).
    f self = ...
Option T: Type = Patch T or NoneType
Option(T).
    f self = ...
Fn T: Type = Patch T -> T
Fn(T).
    f self = ...
Fn2 T, U: Type = Patch T -> U
Fn2(T, U).
    f self = ...

(Int -> Int).f() # 选择了哪一个?
```

在上面的示例中，方法 `f` 会选择哪个补丁?
天真，似乎选择了`Fn T`，但是`Fn2 T，U`也是可以的，`Option T`原样包含`T`，所以任何类型都适用，`Container K，T`也匹配`->(Int, Int)`，即 `Container(`->`, Int)` 为 `Int -> Int`。因此，上述所有四个修复程序都是可能的选择

在这种情况下，根据以下优先标准选择修复程序

* 任何 `K(T)`(例如 `T or NoneType`)优先匹配 `Type -> Type` 而不是 `Type`
* 任何 `K(T, U)`(例如 `T -> U`)优先匹配 `(Type, Type) -> Type` 而不是 `Type`
* 类似的标准适用于种类 3 或更多
* 选择需要较少类型变量来替换的那个。例如，`Int -> Int` 是 `T -> T` 而不是 `K(T, T)`(替换类型变量: K, T)或 `T -> U`(替换类型变量: T, U )。(替换类型变量: T)优先匹配
* 如果更换的次数也相同，则报错为不可选择

---

<span id="1" style="font-size:x-small"><sup>1</sup> 在类型理论符号中，`*=>*` [↩](#f1)</span>

<span id="2" style="font-size:x-small"><sup>2</sup> 可见性等细微差别。[↩](#f2)</span>
