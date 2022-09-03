# 类型变量，量化

类型变量是用于指定子例程参数类型的变量，并指示其类型是可选的（非单相）。首先，作为引入类型变量的动机，让我们考虑返回输入的函数。


```erg
id x: Int = x
```

虽然为类型定义了一个按原样返回输入的<gtr=“25”/>函数，但该函数显然可以为任何类型定义。让我们使用<gtr=“27”/>来表示最大的类。


```erg
id x: Object = x

i = id 1
s = id "foo"
b = id True
```

你现在可以接受任何类型，但有一个问题。返回类型将扩展为。如果输入是<gtr=“29”/>类型，则返回<gtr=“30”/>类型；如果输入是<gtr=“31”/>类型，则返回<gtr=“32”/>类型。


```erg
print! id 1 # <Object object>
id(1) + 1 # TypeError: cannot add `Object` and `Int`
```

若要确保输入类型与返回类型相同，请使用。类型变量在<gtr=“33”/>（类型变量列表）中声明。


```erg
id|T: Type| x: T = x
assert id(1) == 1
assert id("foo") == "foo"
assert id(True) == True
```

我们在函数中称为。虽然有细微差别，但它们相当于其他语言中称为通用的功能。然后，全称量化函数称为<gtr=“36”/>。定义多相关数就像为所有类型定义相同形式的函数一样（由于 Erg 禁止重载，所以下面的代码实际上是不可能写的）。


```erg
id|T: Type| x: T = x
# pseudo code
# ==
id x: Int = x
id x: Str = x
id x: Bool = x
id x: Ratio = x
id x: NoneType = x
...
```

此外，类型变量用于指定类型，因此可以推断为<gtr=“38”/>类型。因此，<gtr=“39”/>可以简单地省略为<gtr=“40”/>。此外，如果你可以推理非类型对象，如<gtr=“41”/>（<gtr=“42”/>），则可以省略它。

另外，如果任何类型太大，也可以给出限制。约束也有好处，例如，子类型规范允许你使用特定的方法。


```erg
# T <: Add
# => TはAddのサブクラス
# => 加算ができる
add|T <: Add| l: T, r: T = l + r
```

在此示例中，必须是<gtr=“44”/>类型的子类，并且实际赋值的<gtr=“45”/>必须与<gtr=“46”/>类型相同。在这种情况下，<gtr=“47”/>是指<gtr=“48”/>或<gtr=“49”/>。因为没有定义<gtr=“50”/>和<gtr=“51”/>的相加，所以弹。

也可以这样打字。


```erg
f|
    Y, Z: Type
    X <: Add Y, O1
    O1 <: Add Z, O2
    O2 <: Add X, _
| x: X, y: Y, z: Z  =
    x + y + z + x
```

如果注释列表变长，最好预先声明。


```erg
f: |Y, Z: Type, X <: Add(Y, O1), O1 <: Add(Z, O2), O2 <: Add(X, O3)| (X, Y, Z) -> O3
f|X, Y, Z| x: X, y: Y, z: Z  =
    x + y + z + x
```

与许多具有通用语言的语言不同，所有声明的类型变量必须在伪参数列表（部分）或其他类型变量的参数中使用。这是 Erg 语言设计提出的要求，即所有类型变量都可以从实际参数中推理。因此，不能推理的信息（如返回类型）是从实际参数传递的。Erg 可以从实际参数中传递类型。


```erg
Iterator T = Trait {
    # 戻り値の型を引数から渡している
    # .collect: |K: Type -> Type| Self(T).({K}) -> K(T)
    .collect(self(T), K: Type -> Type): K(T) = ...
    ...
}

it = [1, 2, 3].iter().map i -> i + 1
it.collect(Array) # [2, 3, 4]
```

类型变量只能在之间声明。但是，声明之后，直到脱离范围为止，可以在任意场所使用。


```erg
f|X|(x: X): () =
    y: X = x.clone()
    log X.__name__
    log X

f 1
# Int
# <class Int>
```

使用时也可以显式单相化，如下所示。


```erg
f: Int -> Int = id|Int|
```

在这种情况下，指定的类型优先于实际参数类型（否则将导致实际参数类型错误的类型错误）。也就是说，如果实际传递的对象可以转换为指定的类型，则会进行转换，否则会导致编译错误。


```erg
assert id(1) == 1
assert id|Int|(1) in Int
assert id|Ratio|(1) in Ratio
# キーワード引数も使える
assert id|T: Int|(1) == 1
id|Int|("str") # TypeError: id|Int| is type `Int -> Int` but got Str
```

当这个语法与内含符号击球时，必须用括起来。


```erg
# {id|Int| x | x <- 1..10}だと{id | ...}だと解釈される
{(id|Int| x) | x <- 1..10}
```

不能声明与已有类型同名的类型变量。这是因为类型变量都是常量。


```erg
I: Type
# ↓ invalid type variable, already exists
f|I: Type| ... = ...
```

## 方法定义中的类型参数

缺省情况下，左侧的类型参数被视为绑定变量。


```erg
K(T: Type, N: Nat) = ...
K(T, N).
    foo(x) = ...
```

如果使用不同的类型变量名称，则会发出警告。


```erg
K(T: Type, N: Nat) = ...
K(U, M). # Warning: K's type variable names are 'T' and 'N'
    foo(x) = ...
```

常量在定义后的所有命名空间中都是相同的，因此也不能用于类型变量名称。


```erg
N = 1
K(N: Nat) = ... # NameError: N is already defined

L(M: Nat) = ...
# M == N == 1のときのみ定義される
L(N).
    foo(self, x) = ...
# 任意のM: Natに対して定義される
L(M).
    .bar(self, x) = ...
```

虽然不能为每个类型参数定义多个参数，但可以定义同名的方法，因为没有指定类型参数的从属类型（非原始卡印）和已指定类型参数的从属类型（原始卡印）没有关系。


```erg
K(I: Int) = ...
K.
    # Kは真の型(原子カインド)ではないので、メソッドを定義できない
    # これはメソッドではない(スタティックメソッドに近い)
    foo(x) = ...
K(0).
    foo(self, x): Nat = ...
```

## 全称型

在上一章中定义的函数可以是任何类型。那么，“函数本身的类型”是什么呢？


```erg
print! classof(id) # |T: Type| T -> T
```

得到了的类型。这被称为<gtr=“60”/>，相当于 ML 中以<gtr=“58”/>形式提供的类型，Haskell 中以<gtr=“59”/>形式提供的类型。为什么要加上“闭合”这个形容词，后面会讲到。

封闭的全称类型是有限制的，只能将子程序类型全称化，即只能将其置于左侧子句中。但这已经足够了。在 Erg 中，子程序是最基本的控制结构，因此当“想要处理任意 X”时，即“想要能够处理任意 X 的子程序”。所以，全称型和多相关数型是一个意思。以后基本上把这种类型称为多相关数类型。

与无名函数一样，多相关数类型具有类型变量名称的任意性，但它们都是等值的。


```erg
assert (|T: Type| T -> T) == (|U: Type| U -> U)
```

在 Lambda 计算中，当α等值时，等号成立。由于类型运算存在一些限制，因此始终可以确定等价性（除非考虑终止性）。

## 多相关数类型的部分类型

多相关数类型可以是任何函数类型。这意味着它与任何函数类型具有子类型关系。让我们来详细了解一下这种关系。

这样的“类型变量在左边定义并在右边使用的类型”称为<gtr=“63”/>。与此相对，<gtr=“62”/>等“类型变量在右边定义和使用的类型”称为<gtr=“64”/>。

打开了的全称型，成为同形的全部的“真的型”的超级型。相反，封闭的全称类型是所有相同类型的“真实类型”的子类型。


```erg
(|T: Type| T -> T) < (Int -> Int) < (T -> T)
```

最好记住关闭的小/打开的大。但怎么会。为了更好地理解每一个实例。


```erg
# id: |T: Type| T -> T
id|T|(x: T): T = x

# iid: Int -> Int
iid(x: Int): Int = x

# 任意の関数をそのまま返す
id_arbitrary_fn|T|(f1: T -> T): (T -> T) = f
# id_arbitrary_fn(id) == id
# id_arbitrary_fn(iid) == iid

# 多相関数をそのまま返す
id_poly_fn(f2: (|T| T -> T)): (|T| T -> T) = f
# id_poly_fn(id) == id
id_poly_fn(iid) # TypeError

# Int型関数をそのまま返す
id_int_fn(f3: Int -> Int): (Int -> Int) = f
# id_int_fn(id) == id|Int|
# id_int_fn(iid) == iid
```

类型的<gtr=“66”/>可以赋给<gtr=“67”/>类型的参数<gtr=“68”/>，因此可以认为是<gtr=“69”/>。相反，<gtr=“70”/>类型的<gtr=“71”/>不能赋给<gtr=“72”/>类型的参数<gtr=“73”/>，但它是<gtr=“76”/>，因为它可以赋给<gtr=“74”/>类型的参数<gtr=“75”/>。因此，确实是<gtr=“77”/>。

## 全称和依赖关系

依存型和全称型（多相关数型）有什么关系，有什么不同呢？依赖类型是一种采用参数的类型，而全称类型是一种为参数提供任意性的类型（在要全称的子程序中）。

重要的是，封闭的全称类型本身没有类型参数。例如，多相关数类型是取多相关数<gtr=“80”/>的类型，其定义是封闭的。不能使用该类型参数<gtr=“79”/>定义方法等。

在 Erg 中，类型本身也是一个值，因此采用参数的类型（例如函数类型）也必须是一个依赖类型。也就是说，多相关数类型既是全称类型，也是依存类型。


```erg
PolyFn = Patch(|T| T -> T)
PolyFn.
    type self = T # NameError: cannot find 'T'
DepFn T = Patch(T -> T)
DepFn.
    type self =
        log "by DepFn"
        T

assert (Int -> Int).type() == Int # by DepFn
assert DepFn(Int).type() == Int # by DepFn
```
