# 函数

函数是一个块，它接受参数并对其进行处理，然后将其作为返回值返回。定义如下。


```erg
add x, y = x + y
# or
add(x, y) = x + y
```

在定义函数时指定的参数通常称为伪参数（parameter）。相反，函数调用过程中传递的参数称为实际参数（argument）。是接受<gtr=“31”/>和<gtr=“32”/>作为假参数，然后返回<gtr=“33”/>的函数。你可以按如下方式调用（应用）定义的函数。


```erg
add 1, 2
# or
add(1, 2)
```

## 冒号样式

函数的调用方式如下：，但如果实际参数太多，一行太长，则可以使用<gtr=“35”/>（冒号）来应用。


```erg
f some_long_name_variable_1 + some_long_name_variable_2, some_long_name_variable_3 * some_long_name_variable_4
```


```erg
f some_long_name_variable_1 + some_long_name_variable_2:
    some_long_name_variable_3 * some_long_name_variable_4
```


```erg
f:
    some_long_name_variable_1 + some_long_name_variable_2
    some_long_name_variable_3 * some_long_name_variable_4
```

上面三个代码都是同一个意思。此样式在使用函数时也很有用。


```erg
result = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
```

在之后，不能写注释以外的代码，必须换行。

## 关键字参数（Keyword Arguments）

如果定义了具有大量参数的函数，则可能会导致传递参数的顺序错误。在这种情况下，使用关键字参数进行调用是安全的。


```erg
f x, y, z, w, v, u: Int = ...
```

上面定义的函数有很多参数，并且排列很难懂。我们不应该做这样的函数，但在使用别人写的代码时可能会碰到这样的代码。因此，我们使用关键字参数。关键字参数的名称优先于顺序，因此即使顺序不正确，也会将值从名称传递到正确的参数。


```erg
f u: 6, v: 5, w: 4, x: 1, y: 2, z: 3
```

请注意，如果在关键字参数和之后立即换行，将被视为冒号应用样式。


```erg
# means `f(x: y)`
f x: y

# means `f(x, y)`
f x:
    y
```

## 默认参数（Default parameters）

如果一个参数在大多数情况下是固定的，并且你想要省略它，则可以使用默认参数。

缺省参数由（or-assign operator）指定。如果未指定<gtr=“40”/>，则将<gtr=“41”/>赋给<gtr=“42”/>。


```erg
math_log x: Ratio, base := math.E = ...

assert math_log(100, 10) == 2
assert math_log(100) == math_log(100, math.E)
```

请注意，不指定参数和赋值是有区别的。


```erg
p! x := 0 = print! x
p!(2) # 2
p!() # 0
p!(None) # None
```

也可以与类型和模式一起使用。


```erg
math_log x, base: Ratio := math.E = ...
f [x, y] := [1, 2] = ...
```

但是，在缺省参数中，不能调用以下过程或赋值可变对象。


```erg
f x := p! 1 = ... # NG
```

此外，不能将刚定义的参数用作传递给缺省参数的值。


```erg
f x := 1, y := x = ... # NG
```

## 可变长度参数

函数将参数作为日志输出，可以接收任意数量的参数。


```erg
log "Hello", "World", "!" # Hello World !
```

如果要定义这样的函数，请将作为参数。这样，参数就可以作为可变长度数组接收。


```erg
f x: ...Int =
    for x, i ->
        log i

# x == [1, 2, 3, 4, 5]
f 1, 2, 3, 4, 5
```

## 多模式函数定义


```erg
fib n: Nat =
    match n:
        0 -> 0
        1 -> 1
        n -> fib(n - 1) + fib(n - 2)
```

如果函数的定义正下方出现，如上面所示，则可以重写如下所示。


```erg
fib 0 = 0
fib 1 = 1
fib(n: Nat): Nat = fib(n - 1) + fib(n - 2)
```

请注意，多模式函数定义不是所谓的过载（多重定义）。一个函数始终只有一个类型。在上面的示例中，必须与<gtr=“48”/>和<gtr=“49”/>具有相同的类型。此外，与<gtr=“50”/>相同，模式匹配从上到下依次进行。

如果存在不同类的混合实例，则必须在最后一个定义中指明函数参数类型为 Or。


```erg
f "aa" = ...
f 1 = ...
# `f x = ...` is invalid
f x: Int or Str = ...
```

它还必须具有包容性，如。


```erg
fib 0 = 0
fib 1 = 1
# PatternError: pattern of fib's parameter is not exhaustive
```

但是，即使在上述情况下，也可以使用下面的显式指定类型来获得全面性。


```erg
fib: 0..1 -> 0..1
fib 0 = 0
fib 1 = 1
# OK
```

## 递归函数

递归函数是定义中包含自身的函数。

作为一个简单的例子，我们尝试定义函数来计算阶乘。阶乘是“乘以所有小于或等于的正数”的计算。5 的阶乘为。


```erg
factorial 0 = 1
factorial 1 = 1
factorial(n: Nat): Nat = n * factorial(n - 1)
```

首先从阶乘定义开始，0 和 1 的阶乘都是 1. 按顺序计算，2 的阶乘为，3 的阶乘为，4 的阶乘为。如果你仔细观察这里，你会发现一个数字 n 的阶乘是它前面的数字 n-1 的阶乘乘以 n。如果你将其放入代码中，则会得到。<gtr=“60”/>是递归函数，因为<gtr=“59”/>的定义包含它自己。

注意，如果未指定类型，则会这样推断。


```erg
factorial: |T <: Sub(Int, T) and Mul(Int, Int) and Eq(Int)| T -> Int
factorial 0 = 1
factorial 1 = 1
factorial n = n * factorial(n - 1)
```

但是，即使可以推理，也应该明确指定递归函数的类型。在上面的示例中，像这样的代码是有效的，


```erg
factorial(-1) == -1 * factorial(-2) == -1 * -2 * factorial(-3) == ...
```

，此计算不会停止。如果不仔细定义值的范围，递归函数可能会陷入无限循环。类型还有助于防止接受不想要的值。

## 编译时函数

如果函数名以大写字母开头，则该函数为编译时函数。所有用户定义的编译时函数的参数都必须是常量，并且必须显式。编译函数能做的事情是有限的。在编译时函数中只能使用常量表达式，即某些运算符（四则运算，比较运算，类型构建运算等）和编译时函数。赋值的参数也必须是常量表达式。相反，计算可以在编译时进行。


```erg
Add(X, Y: Nat): Nat = X + Y
assert Add(1, 2) == 3

Factorial 0 = 1
Factorial(X: Nat): Nat = X * Factorial(X - 1)
assert Factorial(10) == 3628800

math = import "math"
Sin X = math.sin X # ConstantError: this function is not computable at compile time
```

编译时函数通常用于多相类型定义等。


```erg
Option T: Type = T or NoneType
Option: Type -> Type
```

## Appendix：比较函数

Erg 没有为函数定义。那是因为函数的结构等价性判定算法一般不存在。


```erg
f = x: Int -> (x + 1)**2
g = x: Int -> x**2 + 2x + 1

assert f == g # TypeError: cannot compare functions
```

和<gtr=“64”/>总是返回相同的结果，但这是非常困难的。我们得把代数学灌输给编译器。因此，Erg 放弃了整个函数比较，<gtr=“65”/>也会导致编译错误。这是与 Python 不同的规格，需要注意。


```python
# Python, weird example
f = lambda x: x
assert f == f
assert (lambda x: x) != (lambda x: x)
```

## Appendix2：完成（）


```erg
f x: Object = ...
# will be completed to
f(x: Object) = ...

f a
# will be completed to
f(a)

f a, b # TypeError: f() takes 1 positional argument but 2 were given
f(a, b) # TypeError: f() takes 1 positional argument but 2 were given
f((a, b)) # OK
```

函数类型实际上是<gtr=“67”/>的糖衣语法。

<p align='center'>
    <a href='./03_declaration.md'>Previous</a> | <a href='./05_builtin_funcs.md'>Next</a>
</p>
