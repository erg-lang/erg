# 退化（variance）

Erg 可以进行多相型的分型，但是有一部分必须注意的地方。

首先考虑通常的多相型的包含关系。一般情况下，存在容器和代入的类型<gtr=“8”/>，当<gtr=“9”/>时，为<gtr=“10”/>。例如，<gtr=“11”/>。因此，用<gtr=“12”/>定义的方法，也可以使用<gtr=“13”/>。

考虑典型的多相型型。请注意，这一次不考虑元素的数量，因此不是。那么，在<gtr=“16”/>型中存在<gtr=“17”/>和<gtr=“18”/>的方法，分别表示要素的追加、取出。套路是这样的。

Array.push!: Self(T).(T) => NoneTypeArray.pop!: Self(T).() => T

我们可以直观地了解到，

* 当时<gtr=“20”/>OK（将<gtr=“21”/>上传至<gtr=“22”/>即可）
* 当时<gtr=“24”/>为 NG
* 是 NG
* 好的

是。这在类型系统上

* (Self(Object).(Object) => NoneType) < (Self(Str).(Str) => NoneType)
* (Self(Str).() => Str) < (Self(Object).() => Object)

的意思。

前者可能看起来很奇怪。虽然是，但将其作为自变量的函数的包含关系却发生了逆转。在类型理论中，这种关系（的类型关系）称为反变（contravariant），相反，的类型关系称为共变（covariant）。也就是说，可以说函数型是关于自变量的类型的反变，关于返回值的类型的共变。听起来很复杂，但正如刚才看到的那样，如果套用实例来考虑的话，这是一个合理的规则。即便如此，如果还不明白的话，可以考虑如下。

Erg 的设计方针中有“输入的类型大，输出的类型小”。这可以从函数的变性说起。从上面的规则来看，输入型是大的一方整体来说是小的类型。因为通用函数明显比专用函数稀少。而且输出型越小整体越小。

结果上面的方针等于说“函数的类型最小化”。

## 过错变性

Erg 还有一种变性。它是非变性的。这是编入型中等具有的变性。这意味着，关于<gtr=“33”/>的 2 个类型<gtr=“34”/>，即使存在包含关系，也不能在<gtr=“35”/>和<gtr=“36”/>之间进行转换。这是因为是共享参照。有关详细信息，请参见<gtr=“38”/>。

## 变性指定的全称类型

可以指定全称类型的类型变量的上限和下限。


```erg
|A <: T| K(A)
|B :> T| K(B)
```

类型变量列表中的类型变量。在上面的变性说明中，类型变量<gtr=“39”/>是类型<gtr=“40”/>的任何子类，类型变量<gtr=“41”/>是类型<gtr=“42”/>的任何超类。此时，<gtr=“43”/>也称为<gtr=“44”/>的上限型，<gtr=“45”/>的下限型。

还可以叠加退化规范。


```erg
# U < A < T
{... | A <: T; A :> U}
```

下面是使用变性规范的代码示例。


```erg
show|S <: Show| s: S = log s

Nil T = Class(Impl=Phantom T)
Cons T = Class(Nil T or List T)
List T = Class {head = T; rest = Cons T}
List(T).
    push|U <: T|(self, x: U): List T = Self.new {head = x; rest = self}
    upcast(self, U :> T): List U = self
```

## 变性指定

请注意中的示例，我们将更详细地讨论这些示例。为了了解上面的代码，我们需要了解多相型的变性。关于变性，我们在<gtr=“48”/>中进行了详细说明，但目前需要的事实有以下三个：

* 通常的多相型，等对于<gtr=“50”/>共变（<gtr=“51”/>时<gtr=“52”/>）
* 函数与自变量类型<gtr=“54”/>相反（当<gtr=“55”/>时<gtr=“56”/>）
* 函数与返回类型<gtr=“58”/>共变（当<gtr=“59”/>时<gtr=“60”/>）

例如，可以上播到<gtr=“62”/>，<gtr=“63”/>可以上播到<gtr=“64”/>。

现在，我们将考虑如果省略方法的退化规范会发生什么情况。


```erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    # List T can be pushed U if T > U
    push|U|(self, x: U): List T = Self.new {head = x; rest = self}
    # List T can be List U if T < U
    upcast(self, U): List U = self
```

即使在这种情况下，Erg 编译器也可以很好地推论的上限和下限类型。但是，请注意，Erg 编译器并不理解方法的含义。编译器只是根据变量和类型变量的使用方式机械地推理和推导类型关系。

如注释所示，的<gtr=“67”/>类型<gtr=“68”/>是<gtr=“69”/>的子类（如果<gtr=“70”/>，则<gtr=“71”/>等）。即推论为<gtr=“72”/>。此约束禁止更改<gtr=“73”/>参数类型的上传<gtr=“74”/>（e.g.<gtr=“75”/>）。但是，请注意，<gtr=“76”/>约束并没有改变函数类型的包含关系。<gtr=“77”/>这一事实保持不变，只是不能在<gtr=“78”/>方法中执行这样的上播。同样，从<gtr=“79”/>到<gtr=“80”/>的转换在<gtr=“81”/>的约束条件下是可能的，因此可以这样推论退化规范。此约束禁止更改<gtr=“82”/>的返回类型的上传<gtr=“83”/>（e.g.<gtr=“84”/>）。

现在，我想如果我允许这个上传会发生什么情况。让我们来反转退化规范。


```erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    push|U :> T|(self, x: U): List T = Self.new {head = x; rest = self}
    upcast(self, U :> T): List U = self
# TypeWarning: `U` in the `.push` cannot take anything other than `U == T`. Replace `U` with `T`. Or you may have the wrong variance specification.
# TypeWarning: `U` in the `.upcast` cannot take anything other than `U == T`. Replace `U` with `T`. Or you may have the wrong variance specification.
```

只有当同时满足<gtr=“85”/>约束和<gtr=“86”/>退化规范时，才能满足。因此，此指定几乎没有任何意义。实际上，只允许“上播，如<gtr=“88”/>”=“上播，不改变<gtr=“89”/>”。

## Appendix：用户定义的变体

用户定义类型的变性默认为非变。但是，也可以用这一标记轨迹指定变性。如果指定<gtr=“91”/>，则该类型对于<gtr=“92”/>是反变的。如果指定<gtr=“93”/>，则该类型对于<gtr=“94”/>为协变。


```erg
K T = Class(...)
assert not K(Str) <= K(Object)
assert not K(Str) >= K(Object)

InputStream T = Class ..., Impl := Inputs(T)
# Objectを受け入れるストリームは、Strを受け入れるともみなせる
assert InputStream(Str) > InputStream(Object)

OutputStream T = Class ..., Impl := Outputs(T)
# Strを出力するストリームは、Objectを出力するともみなせる
assert OutputStream(Str) < OutputStream(Object)
```
