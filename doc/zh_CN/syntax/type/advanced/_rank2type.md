# rank-2 多相

> ：此文档的信息较旧，通常包含错误。

在 Erg 中，像等可以接受各种类型的函数，即可以定义多相关数。那么，能接受多相关数的函数能被定义吗？例如，这样的函数（请注意，此定义包含错误）。


```erg
# tuple_map(i -> i * 2, (1, "a")) == (2, "aa")我要你成为
tuple_map|T|(f: T -> T, tup: (Int, Str)): (Int, Str) = (f(tup.0), f(tup.1))
```

请注意，由于和的类型不同，因此无名函数并不是单相化一次就结束的。需要进行两次单相化。在至今为止说明的型的范畴中，无法对这样的函数进行定义。因为型变量中没有范围的概念。在此暂时离开类型，确认值水平上的范围概念。


```erg
arr = [1, 2, 3]
arr.map i -> i + 1
```

上述代码中的和<gtr=“18”/>是不同作用域的变量。因此，它们的生存期是不同的（<gtr=“19”/>更短）。

到目前为止的类型，所有的类型变量的生存期都是相同的。也就是说，，<gtr=“21”/>，<gtr=“22”/>同时被确定，以后必须不变。反过来说，如果可以将<gtr=“23”/>看作“内侧范围”中的类型变量，则可以构成<gtr=“24”/>函数。为此准备了<gtr=“25”/>。


```erg
# tuple_map: ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
tuple_map f: (|T: Type| T -> T), tup: (Int, Str) = (f(tup.0), f(tup.1))
assert tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
```

形式的类型称为全称类型（详细情况请参照<gtr=“28”/>）。至今所见的函数是典型的全称函数 = 多相关数。


```erg
id x = x
id: |T: Type| T -> T
```

全称型与函数型构建子之间具有特殊的结合规则，根据结合方法的不同，类型的意义完全不同。

对此，使用单纯的 1 自变量函数进行考虑。


```erg
f1: (T -> T) -> Int | T # 接受任何函数并返回 Int 的函数
f2: (|T: Type| T -> T) -> Int # 接收多相关并返回 Int 的函数
f3: Int -> (|T: Type| T -> T) # 一个函数，接受一个 Int 并返回一个封闭的通用函数
f4: |T: Type|(Int -> (T -> T)) # 同上（首选）
```

和<gtr=“31”/>相同，而<gtr=“32”/>和<gtr=“33”/>却不同，这似乎很奇怪。实际上试着构成这种类型的函数。


```erg
# id: |T: Type| T -> T
id x = x
# same type as `f1`
take_univq_f_and_return_i(_: (|T: Type| T -> T), i: Int): Int = i
# same type as `f2`
take_arbit_f_and_return_i|T: Type|(_: T -> T, i: Int): Int = i
# same type as `f3`
take_i_and_return_univq_f(_: Int): (|T: Type| T -> T) = id
# same type as `f4`
take_i_and_return_arbit_f|T: Type|(_: Int): (T -> T) = id
```

应用之后，就会发现其中的差异。


```erg
_ = take_univq_f_and_return_i(x -> x, 1) # OK
_ = take_univq_f_and_return_i(x: Int -> x, 1) # NG
_ = take_univq_f_and_return_i(x: Str -> x, 1) # NG
_ = take_arbit_f_and_return_i(x -> x, 1) # OK
_ = take_arbit_f_and_return_i(x: Int -> x, 1) # OK
_ = take_arbit_f_anf_return_i(x: Str -> x, 1) # OK

f: |T| T -> T = take_i_and_return_univq_f(1)
g: |T| T -> T = take_i_and_return_arbit_f(1)
assert f == g
f2: Int -> Int = take_i_and_return_univq_f|Int|(1)
g2: Int -> Int = take_i_and_return_arbit_f|Int|(1)
assert f2 == g2
```

开放的多相关数型特别称为。任意函数类型有无限个可能性，如，<gtr=“35”/>，<gtr=“36”/>，<gtr=“37”/>，...等。与此相对，关闭的多相关数类型（返回与参数类型相同的对象）只有<gtr=“38”/>一种。这种类型特别称为。换句话说，可以向<gtr=“39”/>传递<gtr=“40”/>、<gtr=“41”/>、<gtr=“42”/>等的 =<gtr=“43”/>是多相关数，但是可以向<gtr=“44”/>传递的只有<gtr=“45”/>等 =<gtr=“46”/>是多相关数<gtr=“50”/>。但是，像<gtr=“47”/>这样的函数的类型明显与通常的类型不同，需要能够很好地处理这些的新概念。这就是套路的“档次”。

关于等级的定义，首先，未量化的类型，即，<gtr=“52”/>，<gtr=“53”/>，<gtr=“54”/>，<gtr=“55”/>，<gtr=“56”/>等被认为是“等级 0”。


```erg
# KはOptionなどの多項カインド
R0 = (Int or Str or Bool or ...) or (R0 -> R0) or K(R0)
```

其次，将等进行一阶全称量化的类型，或者将其包含在返回值类型中的类型作为“等级 1”。此外，将进行二阶全称量化的类型（以等等级 1 类型为自变量的类型），或将其包含在返回值类型中的类型设为“等级 2”。重复上述操作，定义“秩 N”型。另外，等级 N 型包含 N 以下等级的所有类型。因此，多个等级混合的类型的等级与其中最高的等级相同。


```erg
R1 = (|...| R0) or (R0 -> R1) or K(R1) or R0
R2 = (|...| R1) or (R1 -> R2) or K(R2) or R1
...
Rn = (|...| Rn-1) or (Rn-1 -> Rn) or K(Rn) or Rn-1
```

让我们来看几个例子。


```erg
    (|T: Type| T -> T) -> (|U: Type| U -> U)
=>  R1 -> R1
=>  R1 -> R2
=>  R2

Option(|T: Type| T -> T)
=>  Option(R1)
=>  K(R1)
=>  R1
```

根据定义，是等级 2 型。


```erg
tuple_map:
    ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
=>  (R1, R0) -> R0
=>  R1 -> R2
=>  R2
```

在 Erg 中，可以处理到等级 2 为止的类型（等级 N 型包含 N 以下等级的所有类型，因此正确地说 Erg 的类型都是等级 2 型）。如果试图配置更多类型的函数，则会出现错误。例如，将多相关数作为多相关数处理的函数都需要指定其他自变量的类型。另外，不能构成这样的函数。


```erg
# this is a rank-3 type function
# |X, Y: Type|((|T: Type| T -> T), (X, Y)) -> (X, Y)
generic_tuple_map|X, Y: Type| f: (|T: Type| T -> T), tup: (X, Y) = (f(tup.0), f(tup.1))
```

等级 3 以上的类型在理论上不能决定类型推论的事实已知，类型指定破坏了可以省略的 Erg 的性质，因此被排除。尽管如此，实用需求 2 级基本可以覆盖。
