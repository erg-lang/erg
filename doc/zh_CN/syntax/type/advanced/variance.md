# 变性(逆变与协变)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/variance.md%26commit_hash%3Df4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/variance.md&commit_hash=f4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)

Erg 可以对多态类型进行子类型化，但有一些注意事项

首先，考虑普通多态类型的包含关系。一般来说，有一个容器`K`和它分配的类型`A，B`，当`A < B`时，`K A < K B`
例如，`Option Int < Option Object`。因此，在`Option Object`中定义的方法也可以在`Option Int`中使用

考虑典型的多态类型 `Array!(T)`
请注意，这一次不是 `Array!(T, N)` 因为我们不关心元素的数量
现在，`Array!(T)` 类型具有称为 `.push!` 和 `.pop!` 的方法，分别表示添加和删除元素。这是类型: 

`Array.push!: Self(T).(T) => NoneType`
`Array.pop!: Self(T).() => T`

可以直观地理解:

* `Array!(Object).push!(s)` is OK when `s: Str` (just upcast `Str` to `Object`)
* When `o: Object`, `Array!(Str).push!(o)` is NG
* `Array!(Object).pop!().into(Str)` is NG
* `Array!(Str).pop!().into(Object)` is OK

就类型系统而言，这是

* `(Self(Object).(Object) => NoneType) < (Self(Str).(Str) => NoneType)`
* `(Self(Str).() => Str) < (Self(Object).() => Object)`
方法

前者可能看起来很奇怪。即使是 `Str < Object`，包含关系在将其作为参数的函数中也是相反的
在类型论中，这种关系(`.push!` 的类型关系)称为逆变，反之，`.pop!` 的类型关系称为协变
换句话说，函数类型就其参数类型而言是逆变的，而就其返回类型而言是协变的
这听起来很复杂，但正如我们之前看到的，如果将其应用于实际示例，这是一个合理的规则
如果您仍然不明白，请考虑以下内容

Erg 的设计原则之一是"大输入类型，小输出类型"。这正是函数可变性的情况
看上面的规则，输入类型越大，整体类型越小
这是因为通用函数明显比专用函数少
而且输出类型越小，整体越小

这样一来，上面的策略就相当于说"尽量减少函数的类型"

## 不变性

Erg 有另一个修改。它是不变的
这是对 `SharedCell! T!`等内置类型的修改。这意味着对于两种类型 `T!, U!` 其中 `T! != U!`，在 `SharedCell! T!` 和 `SharedCell!意思是
这是因为`SharedCell！ T!` 是共享参考。有关详细信息，请参阅 [共享参考](shared.md)

## 变异的泛型类型

通用类型变量可以指定其上限和下限

```python
|A <: T| K(A)
|B :> T| K(B)
```

在类型变量列表中，执行类型变量的__variant说明__。在上述变体规范中，类型变量"A"被声明为"T"类型的任何子类，"B"类型被声明为"T"类型的任何父类
在这种情况下，`T` 也称为 `A` 的上部类型和 `B` 的下部类型

突变规范也可以重叠

```python
# U<A<T
{... | A<: T; A :> U}
```

这是使用变量规范的代码示例: 

```python
show|S <: Show| s: S = log s

Nil T = Class(Impl = Phantom T)
Cons T = Class(Nil T or List T)
List T = Class {head = T; rest = Cons T}
List(T).
    push|U <: T|(self, x: U): List T = Self. new {head = x; rest = self}
    upcast(self, U :> T): List U = self
```

## 更改规范

`List T` 的例子很棘手，所以让我们更详细一点
要理解上面的代码，你需要了解多态类型退化。[this section](./variance.md) 中详细讨论了方差，但现在我们需要三个事实: 

* 普通的多态类型，例如`List T`，与`T`是协变的(`List U > List T` when `U > T`)
* 函数 `T -> U` 对于参数类型 `T` 是逆变的(`(S -> U) < (T -> U)` when `S > T`)
* 函数 `T -> U` 与返回类型 `U` 是协变的(`(T -> U) > (T -> S)` 当 `U > S` 时)

例如，`List Int` 可以向上转换为 `List Object`，而 `Obj -> Obj` 可以向上转换为 `Int -> Obj`

现在让我们考虑如果我们省略方法的变量说明会发生什么

```python
...
List T = Class {head = T; rest = Cons T}
List(T).
    # 如果 T > U，列表 T 可以被推入 U
    push|U|(self, x: U): List T = Self. new {head = x; rest = self}
    # List T 可以是 List U 如果 T < U
    upcast(self, U): List U = self
```

即使在这种情况下，Erg 编译器也能很好地推断 `U` 的上下类型
但是请注意，Erg 编译器不理解方法的语义。编译器只是根据变量和类型变量的使用方式机械地推断和派生类型关系

正如评论中所写，放在`List T`的`head`中的`U`类型是`T`的子类(`T: Int`，例如`Nat`)。也就是说，它被推断为 `U <: T`。此约束将 `.push{U}` upcast `(List(T), U) -> List(T) 的参数类型更改为 (List(T), T) -> List(T)`(例如 disallow `列表(整数).push{对象}`)。但是请注意，`U <: T` 约束不会改变函数的类型包含。`(List(Int), Object) -> List(Int) to (List(Int), Int) -> List(Int)` 的事实并没有改变，只是在 `.push` 方法中表示强制转换无法执行
类似地，从 `List T` 到​​ `List U` 的转换可能会受到约束 `U :> T` 的约束，因此可以推断出变体规范。此约束将 `.upcast(U)` 的返回类型更改为向上转换 `List(T) -> List(T) 到 List(T) -> List(T)`(例如 `List(Object) .upcast(Int )`) 被禁止

现在让我们看看如果我们允许这种向上转换会发生什么
让我们反转变性名称

```python
...
List T = Class {head = T; rest = Cons T}
List(T).
    push|U :> T|(self, x: U): List T = Self. new {head = x; rest = self}
    upcast(self, U :> T): List U = self
# 类型警告: `.push` 中的 `U` 不能接受除 `U == T` 之外的任何内容。将"U"替换为"T"
# 类型警告: `.upcast` 中的 `U` 不能接受除 `U == T` 之外的任何内容。将"U"替换为"T"
```

只有当 `U == T` 时，约束 `U <: T` 和修改规范`U :> T` 才满足。所以这个称号没有多大意义
只有"向上转换使得 `U == T`" = "向上转换不会改变 `U` 的位置"实际上是允许的

## 附录: 用户定义类型的修改

默认情况下，用户定义类型的突变是不可变的。但是，您也可以使用 `Inputs/Outputs` 标记Trait指定可变性
如果您指定 `Inputs(T)`，则类型相对于 `T` 是逆变的
如果您指定 `Outputs(T)`，则类型相对于 `T` 是协变的

```python
K T = Class(...)
assert not K(Str) <= K(Object)
assert not K(Str) >= K(Object)

InputStream T = Class ..., Impl := Inputs(T)
# 接受Objects的流也可以认为接受Strs
assert InputStream(Str) > InputStream(Object)

OutputStream T = Class ..., Impl := Outputs(T)
# 输出Str的流也可以认为输出Object
assert OutputStream(Str) < OutputStream(Object)
```