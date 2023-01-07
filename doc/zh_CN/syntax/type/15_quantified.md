# 类型变量

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/15_quantified.md%26commit_hash%3D44d7784aac3550ba97c8a1eaf20b9264b13d4134)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/15_quantified.md&commit_hash=44d7784aac3550ba97c8a1eaf20b9264b13d4134)

类型变量是用于例如指定子程序参数类型的变量，它的类型是任意的(不是单态的)
首先，作为引入类型变量的动机，考虑 `id` 函数，它按原样返回输入

```python
id x: Int = x
```

返回输入的"id"函数是为"Int"类型定义的，但这个函数显然可以为任何类型定义
让我们使用 `Object` 来表示最大的类

```python
id x: Object = x

i = id 1
s = id "foo"
b = id True
```

当然，它现在接受任意类型，但有一个问题: 返回类型被扩展为 `Object`。返回类型扩展为 `Object`
如果输入是"Int"类型，我想查看返回类型"Int"，如果输入是"Str"类型，我想查看"Str"

```python
print! id 1 # <Object object>
id(1) + 1 # 类型错误: 无法添加 `Object` 和 `Int
```

要确保输入的类型与返回值的类型相同，请使用 __type 变量__
类型变量在`||`(类型变量列表)中声明

```python
id|T: Type| x: T = x
assert id(1) == 1
assert id("foo") == "foo"
assert id(True) == True
```

这称为函数的 __universal quantification(泛化)__。有细微的差别，但它对应于其他语言中称为泛型的函数。泛化函数称为__多态函数__
定义一个多态函数就像为所有类型定义一个相同形式的函数(Erg 禁止重载，所以下面的代码真的不能写)

```python
id|T: Type| x: T = x
# 伪代码
id x: Int = x
id x: Str = x
id x: Bool = x
id x: Ratio = x
id x: NoneType = x
...
```

此外，类型变量"T"可以推断为"Type"类型，因为它在类型规范中使用。所以 `|T: Type|` 可以简单地缩写为 `|T|`
你也可以省略`|T, N| 脚; N]` 如果可以推断它不是类型对象(`T: Type, N: Nat`)

如果类型对于任意类型来说太大，您也可以提供约束
约束也有优势，例如，子类型规范允许使用某些方法

```python
# T <: Add
# => T 是 Add 的子类
# => 可以做加法
add|T <: Add| l: T, r: T = l + r
```

在本例中，`T` 必须是`Add` 类型的子类，并且要分配的`l` 和`r` 的实际类型必须相同
在这种情况下，"T"由"Int"、"Ratio"等满足。因此，例如，"Int"和"Str"的添加没有定义，因此被拒绝

您也可以像这样键入它

```python
f|
    Y, Z: Type
    X <: Add Y, O1
    O1 <: Add Z, O2
    O2 <: Add X, _
| x: X, y: Y, z: Z =
    x + y + z + x
```

如果注释列表很长，您可能需要预先声明它

```python
f: |Y, Z: Type, X <: Add(Y, O1), O1 <: Add(Z, O2), O2 <: Add(X, O3)| (X, Y, Z) -> O3
f|X, Y, Z| x: X, y: Y, z: Z =
    x + y + z + x
```

与许多具有泛型的语言不同，所有声明的类型变量都必须在临时参数列表(`x: X, y: Y, z: Z` 部分)或其他类型变量的参数中使用
这是 Erg 语言设计的一个要求，即所有类型变量都可以从真实参数中推断出来
因此，无法推断的信息，例如返回类型，是从真实参数传递的； Erg 允许从实参传递类型

```python
Iterator T = Trait {
    # 从参数传递返回类型
    # .collect: |K: Type -> Type| Self(T). ({K}) -> K(T)
    .collect(self, K: Type -> Type): K(T) = ...
    ...
}

it = [1, 2, 3].iter().map i -> i + 1
it.collect(Array) # [2, 3, 4].
```

类型变量只能在 `||` 期间声明。但是，一旦声明，它们就可以在任何地方使用，直到它们退出作用域

```python
f|X|(x: X): () =
    y: X = x.clone()
    log X.__name__
    log X

f 1
# Int
# <class Int>
```

您也可以在使用时明确单相如下

```python
f: Int -> Int = id|Int|
```

在这种情况下，指定的类型优先于实际参数的类型(匹配失败将导致类型错误，即实际参数的类型错误)
即如果传递的实际对象可以转换为指定的类型，则进行转换； 否则会导致编译错误

```python
assert id(1) == 1
assert id|Int|(1) in Int
assert id|Ratio|(1) in Ratio
# 你也可以使用关键字参数
assert id|T: Int|(1) == 1
id|Int|("str") # 类型错误: id|Int| is type `Int -> Int`但得到了 Str
```

当此语法与理解相冲突时，您需要将其括在 `()` 中

```python
# {id|Int| x | x <- 1..10} 将被解释为 {id | ...}
{(id|Int| x) | x <- 1..10}
```

不能使用与已存在的类型相同的名称来声明类型变量。这是因为所有类型变量都是常量

```python
I: Type
# ↓ 无效类型变量，已经存在
f|I: Type| ... = ...
```

## 在方法定义中输入参数

默认情况下，左侧的类型参数被视为绑定变量

```python
K(T: Type, N: Nat) = ...
K(T, N).
    foo(x) = ...
```

使用另一个类型变量名称将导致警告

```python
K(T: Type, N: Nat) = ...
K(U, M). # 警告: K 的类型变量名是 'T' 和 'N'
    foo(x) = ...
```

自定义以来，所有命名空间中的常量都是相同的，因此它们当然不能用于类型变量名称

```python
N = 1
K(N: Nat) = ... # 名称错误: N 已定义

L(M: Nat) = ...
# 仅当 M == N == 1 时才定义
L(N).
    foo(self, x) = ...
# 为任何定义 M: Nat
L(M).
    .bar(self, x) = ...
```

每个类型参数不能有多个定义，但可以定义具有相同名称的方法，因为未分配类型参数的依赖类型(非原始类型)和分配的依赖类型(原始类型)之间没有关系 )

```python
K(I: Int) = ...
K.
    # K 不是真正的类型(atomic Kind)，所以我们不能定义方法
    # 这不是方法(更像是静态方法)
    foo(x) = ...
K(0).
    foo(self, x): Nat = ...
```

## For-all类型

上一节中定义的 `id` 函数是一个可以是任何类型的函数。那么 `id` 函数本身的类型是什么?

```python
print! classof(id) # |T: Type| T -> T
```

我们得到一个类型`|T: Type| T -> T`。这称为一个 __封闭的全称量化类型/全称类型__，即`['a. ...]'` 在 ML 和 `forall t. ...` 在 Haskell 中。为什么使用形容词"关闭"将在下面讨论

封闭的全称量化类型有一个限制: 只有子程序类型可以被通用量化，即只有子程序类型可以放在左子句中。但这已经足够了，因为子程序是 Erg 中最基本的控制结构，所以当我们说"我要处理任意 X"时，即我想要一个可以处理任意 X 的子程序。所以，量化类型具有相同的含义 作为多态函数类型。从现在开始，这种类型基本上被称为多态函数类型

与匿名函数一样，多态类型具有任意类型变量名称，但它们都具有相同的值

```python
assert (|T: Type| T -> T) == (|U: Type| U -> U)
```

当存在 alpha 等价时，等式得到满足，就像在 lambda 演算中一样。由于对类型的操作有一些限制，所以总是可以确定等价的(如果我们不考虑 stoppage 属性)

## 多态函数类型的子类型化

多态函数类型可以是任何函数类型。这意味着与任何函数类型都存在子类型关系。让我们详细看看这种关系

类型变量在左侧定义并在右侧使用的类型，例如 `OpenFn T: Type = T -> T`，称为 __open 通用类型__
相反，在右侧定义和使用类型变量的类型，例如 `ClosedFn = |T: Type| T -> T`，被称为 __封闭的通用类型__

开放通用类型是所有同构"真"类型的父类型。相反，封闭的通用类型是所有同构真类型的子类型

```python
(|T: Type| T -> T) < (Int -> Int) < (T -> T)
```

您可能还记得封闭的较小/开放的较大
但为什么会这样呢? 为了更好地理解，让我们考虑每个实例

```python
# id: |T: Type| T -> T
id|T|(x: T): T = x

# iid: Int -> Int
iid(x: Int): Int = x

# 按原样返回任意函数
id_arbitrary_fn|T|(f1: T -> T): (T -> T) = f
# id_arbitrary_fn(id) == id
# id_arbitrary_fn(iid) == iid

# return the poly correlation number as it is
id_poly_fn(f2: (|T| T -> T)): (|T| T -> T) = f
# id_poly_fn(id) == id
id_poly_fn(iid) # 类型错误

# 按原样返回 Int 类型函数
id_int_fn(f3: Int -> Int): (Int -> Int) = f
# id_int_fn(id) == id|Int|
# id_int_fn(iid) == iid
```

由于 `id` 是 `|T: Type| 类型T -> T`，可以赋值给`Int-> Int`类型的参数`f3`，我们可以考虑`(|T| T -> T) < (Int -> Int)`
反之，`Int -> Int`类型的`iid`不能赋值给`(|T| T -> T)`类型的参数`f2`，但可以赋值给`(|T| T -> T)`的参数`f1`输入 `T -> T`，所以 `(Int -> Int) < (T -> T)`
因此，确实是`(|T| T -> T) < (Int -> Int) < (T -> T)`

## 量化类型和依赖类型

依赖类型和量化类型(多态函数类型)之间有什么关系，它们之间有什么区别?
我们可以说依赖类型是一种接受参数的类型，而量化类型是一种赋予参数任意性的类型

重要的一点是封闭的多态类型本身没有类型参数。例如，多态函数类型`|T| T -> T` 是一个接受多态函数 __only__ 的类型，它的定义是封闭的。您不能使用其类型参数`T`来定义方法等

在 Erg 中，类型本身也是一个值，因此带参数的类型(例如函数类型)可能是依赖类型。换句话说，多态函数类型既是量化类型又是依赖类型

```python
PolyFn = Patch(|T| T -> T)
PolyFn.
    type self = T # 名称错误: 找不到"T"
DepFn T = Patch(T -> T)
DepFn.
    type self =
        log "by DepFn"
        T

assert (Int -> Int).type() == Int # 由 DepFn
assert DepFn(Int).type() == Int # 由 DepFn
```
<p align='center'>
    <a href='./14_dependent.md'>上一页</a> | <a href='./16_subtyping.md'>下一页</a>
</p>