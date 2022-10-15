# rank-2 多态性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/_rank2type.md%26commit_hash%3D13f2d31aee9012f60b7a40d4b764921f1419cdfe)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/_rank2type.md&commit_hash=13f2d31aee9012f60b7a40d4b764921f1419cdfe)

> __Warning__: 本文档已过时，一般包含错误

Erg 允许您定义接受各种类型的函数，例如 `id|T|(x: T): T = x`，即多相关
那么，我们可以定义一个接受多相关的函数吗?
比如这样的函数(注意这个定义是错误的): 

```python
# 我想要 tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
tuple_map|T|(f: T -> T, tup: (Int, Str)): (Int, Str) = (f(tup.0), f(tup.1))
```

注意 `1` 和 `"a"` 有不同的类型，所以匿名函数一次不是单态的。需要单相两次
这样的函数不能在我们目前讨论的类型范围内定义。这是因为类型变量没有范围的概念
让我们暂时离开类型，看看值级别的范围概念

```python
arr = [1, 2, 3]
arr.map i -> i + 1
```

上面代码中的 `arr` 和 `i` 是不同作用域的变量。因此，每个寿命都是不同的(`i` 更短)

到目前为止，所有类型变量的类型都具有相同的生命周期。换句话说，‘T’、‘X’和‘Y’必须同时确定，之后保持不变
反之，如果我们可以将 `T` 视为"内部作用域"中的类型变量，我们可以组成一个 `tuple_map` 函数。__Rank 2 type__ 就是为此目的而准备的

```python
# tuple_map: ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
tuple_map f: (|T: Type| T -> T), tup: (Int, Str) = (f(tup.0), f(tup.1))
assert tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
```

`{(type) | 形式的类型 (类型变量列表)}` 被称为通用类型(详见[通用类型](../15_quantified.md))
目前我们看到的`id`函数是一个典型的通用函数=多相关函数

```python
id x = x
id: |T: Type| T -> T
```

通用类型与函数类型构造函数`->`的关联有特殊的规则，根据关联的方式，类型的语义是完全不同的

用简单的单参数函数来考虑这一点

```python
f1: (T -> T) -> 整数 | T # 接受任何函数并返回 Int 的函数
f2: (|T: Type| T -> T) -> Int # 接收多相关并返回 Int 的函数
f3: Int -> (|T: Type| T -> T) # 一个函数，接受一个 Int 并返回一个封闭的通用函数
f4: |T: Type|(Int -> (T -> T)) # 同上(首选)
```

`f1` 和 `f2` 不同，而 `f3` 和 `f4` 相同，这似乎很奇怪。让我们实际构造一个这种类型的函数

```python
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

After applying it, you will notice the difference.

```python
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

开放的多相关函数类型具体称为 __任意函数类型__。任意函数类型有无数种可能性: `Int -> Int`、`Str -> Str`、`Bool -> Bool`、`|T: Type| T -> T`, ... 是
另一方面，只有一个封闭的(返回与参数相同类型的对象)多态类型`|T: Type| T -> T`。这种类型被专门称为 __多态函数类型__
也就是说，`f1`可以通过`x: Int -> x+1`、`x: Bool -> not x`、`x -> x`等=`f1`是一个多相关数是的，但是您只能将 `x -> x` 等传递给 `f2` = `f2` 不是 __多元相关__
但是像`f2`这样的函数类型明显不同于普通类型，我们需要新的概念来处理它们。那是类型的"等级"

关于rank的定义，没有量化的类型，如`Int`、`Str`、`Bool`、`T`、`Int -> Int`、`Option Int`等，都被视为"rank" 0"

```python
# K 是多项式类型，例如 Option
R0 = (Int or Str or Bool or ...) or (R0 -> R0) or K(R0)
```

接下来，具有一阶全称的类型，例如`|T| T -> T`，或者在返回值类型中包含它们的类型是"rank 1"
此外，具有二阶全称量化的类型(具有 rank 1 类型作为参数的类型，例如 `(|T| T -> T) -> Int`)或将它们包含在返回类型中的类型称为"rank 2 "
重复上述以定义"Rank N"类型。此外，秩-N 类型包括秩为N 或更少的所有类型。因此，混合等级的类型与其中最高的等级相同

```python
R1 = (|...| R0) or (R0 -> R1) or K(R1) or R0
R2 = (|...| R1) or (R1 -> R2) or K(R2) or R1
...
Rn = (|...| Rn-1) or (Rn-1 -> Rn) or K(Rn) or Rn-1
```

让我们看看例子: 

```python
    (|T: Type| T -> T) -> (|U: Type| U -> U)
=> R1 -> R1
=> R1 -> R2
=> R2

Option(|T: Type| T -> T)
=> Option(R1)
=> K(R1)
=> R1
```

根据定义，`tuple_map` 是 rank-2 类型

```python
tuple_map:
    ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
=> (R1, R0) -> R0
=> R1 -> R2
=> R2
```

Erg 最多可以处理 rank 2 的类型(因为 rank N 类型包括所有 rank N 或更少的类型，确切地说，所有 Erg 类型都是 rank 2 类型)。试图构造更多类型的函数是错误的
例如，所有处理多相关的函数都需要指定其他参数类型。而且，这样的功能是不可配置的

```python
# 这是一个 rank-3 类型的函数
# |X, Y: Type|((|T: Type| T -> T), (X, Y)) -> (X, Y)
generic_tuple_map|X, Y: Type| f: (|T: Type| T -> T), tup: (X, Y) = (f(tup.0), f(tup.1))
```

众所周知，具有 3 级或更高等级的类型在理论上无法通过类型推断来确定。然而，大多数实际需求可以被等级 2 类型覆盖。