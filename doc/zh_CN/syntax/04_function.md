＃ 功能

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/04_function.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/04_function.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

函数是一个块，它接受一个“参数”，对其进行处理，并将其作为“返回值”返回。 定义如下。

```python
add x, y = x + y
# 或者
add(x, y) = x + y
```

在函数名之后指定的名称称为参数。
相反，传递给函数的对象称为参数。
函数 `add` 是一个以 `x` 和 `y` 作为参数并返回它们之和的函数，`x + y`。
可以按如下方式调用(应用/调用)定义的函数。

```python
add 1, 2
# or
add(1, 2)
```

## 冒号应用风格

函数像`f x, y, ...`一样被调用，但是如果单行参数太多，可以使用`:`(冒号)来应用它们。

```python
f some_long_name_variable_1 + some_long_name_variable_2, some_long_name_variable_3 * some_long_name_variable_4
```

```python
f some_long_name_variable_1 + some_long_name_variable_2:
    some_long_name_variable_3 * some_long_name_variable_4
```

```python
f:
    some_long_name_variable_1 + some_long_name_variable_2
    some_long_name_variable_3 * some_long_name_variable_4
```

以上三个代码的含义相同。 例如，这种风格在使用 `if` 函数时也很有用

```python
result = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
```

在 `:` 之后，除了注释之外，不得编写任何代码，并且必须始终在新行上

## 关键字参数

如果使用大量参数定义函数，则存在以错误顺序传递参数的危险。
在这种情况下，使用关键字参数调用函数是安全的。

```python
f x, y, z, w, v, u: Int = ...
```

上面定义的函数有很多参数，并且排列顺序混乱。 您不应该创建这样的函数，但是在使用别人编写的代码时可能会遇到这样的代码。 因此，我们使用关键字参数。 如果使用关键字参数，则值会从名称传递到正确的参数，即使它们的顺序错误。

```python
f u: 6, v: 5, w: 4, x: 1, y: 2, z: 3
```

请注意，紧跟在 `:` 之后的关键字参数和新行被视为冒号调用样式

```python
# 意思是 `f(x: y)`
f x: y

# 意思是 `f(x, y)`
f x:
    y
```

## 定义错误参数

当某些参数大部分是固定的并且您希望能够省略它们时，使用默认参数。

默认参数由`:=`(walrus运算符)指定。 如果未指定 `base`，则将 `math.E` 分配给 `base`。

```python
math_log x: Ratio, base := math.E = ...

assert math_log(100, 10) == 2
assert math_log(100) == math_log(100, math.E)
```

请注意，不指定参数和指定`None`是有区别的

```python
p! x := 0 = print!
p!(2) # 2
p!() # 0
p!(None) # None
```

也可以与类型规范和模式一起使用

```python
math_log x, base: Ratio := math.E = ...
f [x, y] := [1, 2] = ...
```

但是，在默认参数中，不能调用过程(稍后描述)或分配可变对象

```python
f x := p! 1 = ... # NG
```

此外，刚刚定义的参数不能用作传递给默认参数的值

```python
f x := 1, y := x = ... # NG
```

## 可变长度参数

输出其参数的日志(记录)的 `log` 函数可以采用任意数量的参数。

```蟒蛇
记录“你好”、“世界”、“！” ＃ 你好世界 ！
```

要定义这样的函数，请将 `...` 添加到参数中。 这样，函数将参数作为可变长度数组接收

```python
f ...x =
    for x, i ->
        log i

# x == [1, 2, 3, 4, 5]
f 1, 2, 3, 4, 5
```

## 具有多种模式的函数定义

```python
fib n: Nat =
    match n:
        0 -> 0
        1 -> 1
        n -> fib(n - 1) + fib(n - 2)
```

像上面这样的函数，其中 `match` 直接出现在定义下，可以重写如下

```python
fib 0 = 0
fib 1 = 1
fib(n: Nat): Nat = fib(n - 1) + fib(n - 2)
```

注意一个函数定义有多个模式不是所谓的重载(multiple definition)； 一个函数只有一个定义。 在上面的示例中，“n”必须与“0”或“1”属于同一类型。 此外，与 `match` 一样，模式匹配是从上到下完成的。

如果不同类的实例混合在一起，最后一个定义必须指定函数参数的类型为`Or`

```python
f "aa" = ...
f 1 = ...
# `f x = ... ` 无效
f x: Int or Str = ...
```

此外，像 `match` 一样，它也必须是详尽的。

```python
fib 0 = 0
fib 1 = 1
# 模式错误：fib 参数的模式并不详尽
```

但是，可以通过使用稍后描述的 [refinement type](./type/12_refinement.md) 显式指定类型来使其详尽无遗。

```python
fib: 0..1 -> 0..1
fib 0 = 0
fib 1 = 1
# OK
```

## 递归函数

递归函数是在其定义中包含自身的函数。

作为一个简单的例子，让我们定义一个执行阶乘计算的函数`factorial`。 阶乘是“将所有小于或等于的正数相乘”的计算。
5 的阶乘是 `5*4*3*2*1 == 120`。

```python
factorial 0 = 1
factorial 1 = 1
factorial(n: Nat): Nat = n * factorial(n - 1)
```

首先，从阶乘的定义来看，0和1的阶乘都是1。
反过来，2的阶乘是`2*1 == 2`，3的阶乘是`3*2*1 == 6`，4的阶乘是`4*3*2*1 == 24 `。
如果我们仔细观察，我们可以看到一个数 n 的阶乘是前一个数 n-1 乘以 n 的阶乘。
将其放入代码中，我们得到 `n * factorial(n - 1)`。
由于 `factorial` 的定义包含自身，`factorial` 是一个递归函数。

提醒一下，如果您不添加类型规范，则会这样推断。

```python
factorial: |T <: Sub(Int, T) and Mul(Int, Int) and Eq(Int)| T -> Int
factorial 0 = 1
factorial 1 = 1
factorial n = n * factorial(n - 1)
```

但是，即使您可以推理，您也应该明确指定递归函数的类型。 在上面的例子中，像“factorial(-1)”这样的代码可以工作，但是

```python
factorial(-1) == -1 * factorial(-2) == -1 * -2 * factorial(-3) == ...
```

并且这种计算不会停止。 递归函数必须仔细定义值的范围，否则您可能会陷入无限循环。
所以类型规范也有助于避免接受意外的值。

## 编译时函数

函数名以大写字母开头，表示编译时函数。 用户定义的编译时函数必须将所有参数作为常量，并且必须指定它们的类型。
编译时函数的功能有限。 在编译时函数中只能使用常量表达式，即只有一些运算符(例如求积、比较和类型构造操作)和编译时函数。 要传递的参数也必须是常量表达式。
作为回报，优点是计算可以在编译时完成。

```python
Add(X, Y: Nat): Nat = X + Y
assert Add(1, 2) == 3

Factorial 0 = 1
Factorial(X: Nat): Nat = X * Factorial(X - 1)
assert Factorial(10) == 3628800

math = import "math"
Sin X = math.sin X # 常量错误：此函数在编译时不可计算
```

编译时函数也用于多态类型定义。

```python
Option T: Type = T or NoneType
Option: Type -> Type
```

## 附录：功能对比

Erg 没有为函数定义 `==`。 这是因为通常没有函数的结构等价算法。

```python
f = x: Int -> (x + 1)**2
g = x: Int -> x**2 + 2x + 1

assert f == g # 类型错误：无法比较函数
```

尽管 `f` 和 `g` 总是返回相同的结果，但要做出这样的决定是极其困难的。 我们必须向编译器教授代数。
所以 Erg 完全放弃了函数比较，并且 `(x -> x) == (x -> x)` 也会导致编译错误。 这是与 Python 不同的规范，应该注意

```python
# Python，奇怪的例子
f = lambda x: x
assert f == f
assert (lambda x: x) ! = (lambda x: x)
```

## Appendix2: ()-completion

```python
f x: Object = ...
# 将完成到
f(x: Object) = ...

f a
# 将完成到
f(a)

f a, b # 类型错误：f() 接受 1 个位置参数，但给出了 2 个
f(a, b) # # 类型错误：f() 接受 1 个位置参数，但给出了 2 个
f((a, b)) # OK
```

函数类型`T -> U`实际上是`(T,) -> U`的语法糖。

<p align='center'>
    <a href='./03_declaration.md'>上一页</a> | <a href='./05_builtin_funcs.md'>下一页</a>
</p>
