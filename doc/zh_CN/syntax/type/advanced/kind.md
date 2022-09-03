# kind（Kind）

Erg 中全部都是定型的。套路本身也不例外。表示“类型的类型”的是。例如，<gtr=“14”/>属于<gtr=“15”/>，<gtr=“16”/>属于<gtr=“17”/>。<gtr=“18”/>是最简单的kind<gtr=“22”/>。在型理论性的记法中，<gtr=“19”/>与<gtr=“20”/>相对应。

在kind这个概念中，实用上重要的是一项以上的kind（多项kind）。1 项的kind度，例如等属于它。1 项kind度表示为<gtr=“31”/>。<gtr=“25”/>和<gtr=“26”/>等的<gtr=“32”/>是特别将类型作为自变量的多项关系。正如<gtr=“27”/>这一表记所示，实际上<gtr=“28”/>是接受<gtr=“29”/>这一类型并返回<gtr=“30”/>这一类型的函数。但是，由于该函数不是通常意义上的函数，因此通常被称为 1 项kind（unary kind）。

另外，作为无名函数运算符的本身也可以看作是接收类型并返回类型时的关系。

另外，请注意，非原子kind的kind不是套路。就像是数值但<gtr=“35”/>不是数值一样，<gtr=“36”/>是类型但<gtr=“37”/>不是类型。等有时也被称为类型构建子。


```erg
assert not Option in Type
assert Option in Type -> Type
```

因此，下面的代码会出现错误。在 Erg 中可以定义方法的只有原子kind度，方法的第一自变量以外的地方不能使用这个名字。


```erg
# K is an unary kind
K: Type -> Type
K T = Class ...
K.
    foo x = ... # OK，这就像是所谓的静态方法
    bar self, x = ... # TypeError: cannot define a method to a non-type object
K(T).
    baz self, x = ... # OK
```

二进制或更高类型的示例是 `{T: U}`(: `(Type, Type) -> Type`), `(T, U, V)`(: `(Type, Type, Type) - > Type `), ... 等等。

还有一个零项类型`() -> Type`。 这有时等同于类型论中的原子类型，但在 Erg 中有所区别。 一个例子是“类”。


```erg
Nil = Class()
```

## kind包含关系

多项关系之间也有部分型关系，原来部分关系。


```erg
K T = ...
L = Inherit K
L <: K
```

也就是说，对于任何，<gtr=“47”/>都是<gtr=“48”/>，反之亦然。


```erg
∀T. L T <: K T <=> L <: K
```

## 高阶kind

还有一种叫做高阶kind（higher-order kind）。这是与高阶函数相同概念的kind，是接受kind本身的kind。等是高阶kind度。尝试定义属于高阶kind度的对象。


```erg
IntContainerOf K: Type -> Type = K Int
assert IntContainerOf Option == Option Int
assert IntContainerOf Result == Result Int
assert IntContainerOf in (Type -> Type) -> Type
```

多项kind度的约束变量通常表示为 K，L，...等（K 是 Kind 的 K）。

## 套管

在型理论中，有一个叫做记录的概念。这与 Erg 的记录基本相同。


```erg
# This is a record, and it corresponds to what is called a record in type theory
{x = 1; y = 2}
```

当记录的值全部为类型时，它被称为记录类型，是类型的一种。


```erg
assert {x = 1; y = 2} in {x = Int; y = Int}
```

记录类型用于输入记录。善于体谅的人可能会认为，应该有“唱片kind”来定型唱片型。实际上，它是存在的。


```erg
log Typeof {x = Int; y = Int} # {{x = Int; y = Int}}
```

像这样的类型就是唱片kind。这不是特别的记法。它是只以为要素的枚举型。


```erg
Point = {x = Int; y = Int}
Pointy = {Point}
```

记录kind的重要特性在于，当<gtr=“54”/>时，<gtr=“55”/>。这从列举型实际上是筛子型的糖衣句法就可以看出。


```erg
# 通常のオブジェクトでは{c} == {X: T | X == c}だが、
# 型の場合等号が定義されない場合があるので|T| == {X | X <: T}となる
{Point} == {P | P <: Point}
```

型限制中的实际上是的糖衣句法。这种类型的组套即kind一般被称为组套kind。设定kind也出现在 Iterator 模式中。


```erg
Iterable T = Trait {
    .Iterator = {Iterator}
    .iter = Self(T).() -> Self.Iterator T
}
```

## 多项关系型推理


```erg
Container K: Type -> Type, T: Type = Patch K(T, T)
Container(K).
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

(Int -> Int).f() # どれが選択される?
```

在上面的例子中，方法选择哪个补丁呢？简单地说，<gtr=“61”/>被认为是可以选择的，但<gtr=“62”/>也有可能，<gtr=“63”/>包含<gtr=“64”/>的原样，因此任意类型都适用，<gtr=“65”/>也是<gtr=“58”/>，即<gtr=“59”/>与<gtr=“66”/>相匹配。因此，上面的 4 个补丁都可以作为选择。

在这种情况下，根据以下优先标准选择补丁。

* 任何（e.g.<gtr=“68”/>）比<gtr=“69”/>优先匹配<gtr=“70”/>。
* 任何（e.g.<gtr=“72”/>）比<gtr=“73”/>优先匹配<gtr=“74”/>。
* 同样的标准适用于 3 项以上的kind度。
* 选择替换类型变量较少的变量。例如，优先匹配<gtr=“78”/>（替换类型变量：T），而不是<gtr=“76”/>（替换类型变量：K，T）或<gtr=“77”/>（替换类型变量：T，U）。
* 如果替换数相同，则错误为“无法选择”。

---

<span id="1" style="font-size:x-small">在1<gtr=“82”/>型理论的记法中<gtr=“79”/><gtr=“80”/></span>

<span id="2" style="font-size:x-small">2<gtr=“85”/>存在可视性等微妙的差异。<gtr=“83”/></span>
