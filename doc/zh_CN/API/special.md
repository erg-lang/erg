# 特殊形式

特殊形式是不能在 Erg 类型系统中表达的运算符、子程序（等等）。它被`包围，但实际上无法捕获。
此外，为方便起见，还出现了“Pattern”、“Body”和“Conv”等类型，但不存在此类类型。它的含义也取决于上下文。

## `=`(pat: Pattern, body: Body) -> NoneType

将 body 分配给 pat 作为变量。如果变量已存在于同一范围内或与 pat 不匹配，则引发错误。
它还用于记录属性定义和默认参数。

```erg
record = {i = 1; j = 2}
f(x: Int, y = 2) = ...
```

当主体是类型或函数时，`=` 具有特殊行为。
左侧的变量名嵌入到右侧的对象中。

```erg
print! Class() # <class <lambda>>
print! x: Int -> x + 1 # <function <lambda>>
C = Class()
print! c # <class C>
f = x: Int -> x + 1
print! f # <function f>
g x: Int = x + 1
print! g # <function g>
K X: Int = Class(...)
print! K # <kind K>
L = X: Int -> Class(...)
print! L # <kind L>
```

`=` 运算符的返回值为“未定义”。
函数中的多个赋值和 `=` 会导致语法错误。

``` 呃
i = j = 1 # SyntaxError: 不允许多次赋值
print!(x=1) # SyntaxError: cannot use `=` in function arguments
# 提示：您的意思是关键字参数（`x: 1`）吗？
if True, do:
    i = 0 # SyntaxError: 块不能被赋值表达式终止
```

## `->`(pat: Pattern, body: Body) -> Func

生成匿名函数，函数类型。

## `=>`(pat: Pattern, body: Body) -> Proc

生成匿名过程，过程类型。

## `:`(subject, T)

确定主题是否与 T 匹配。如果它们不匹配，则抛出编译错误。

```erg
a: Int
f x: Int, y: Int = x / y
```

也用于 `:` 应用样式。

```erg
f x:
    y
    z
```

像`:`和`=`一样，运算的结果是不确定的。

```erg
_ = x: Int # 语法错误：
print!(x: Int) # 语法错误：
```

## `.`(obj, attr)

读取obj的属性。
`x.[y, z]` 将 x 的 y 和 z 属性作为数组返回。

## `|>`(obj, c: Callable)

执行`c(obj)`。 `x + y |>.foo()` 与 `(x + y).foo()` 相同。

### (x: Option T)`?` -> T | T 

后缀运算符。如果出现错误，请立即调用 `x.unwrap()` 和 `return`。

## match(obj, ...lambdas: Lambda)

对于 obj，执行与模式匹配的 lambda。

```erg
match [1, 2, 3]:
  (l: Int) -> log "this is type of Int"
  [[a], b] -> log a, b
  [...a] -> log a
# (1, 2, 3)
```

## del(x: ...T) -> NoneType | T

删除变量“x”。但是，无法删除内置对象。

```erg
a = 1
del a # OK

del True # SyntaxError: cannot delete a built-in object
```

## do(body: Body) -> Func

生成一个不带参数的匿名函数。 `() ->` 的语法糖。

## do!(body: Body) -> Proc

生成不带参数的匿名过程。 `() =>` 的语法糖。

## `else`(l, r) -> Choice

创建一个由两对组成的类元组结构，称为 Choice 对象。
`l, r` 被懒惰地评估。也就是说，只有在调用 .get_then 或 .get_else 时才会计算表达式。

```erg
choice = 1 else 2
assert choice.get_then() == 1
assert choice.get_else() == 2
assert True.then(choice) == 1
```

## 集合运算符

### `[]`(...objs)

从参数创建一个数组或从可选参数创建一个字典。

### `{}`(...objs)

从参数创建一个集合。

### `{}`(...fields: ((Field, Value); N))

生成记录。

### `{}`(layout, ...names, ...preds)

生成筛型，等级2型。

### `...`

展开嵌套集合。它也可以用于模式匹配。

``` erg
[x, ...y] = [1, 2, 3]
assert x == 1 and y == [2, 3]
assert [x, ...y] == [1, 2, 3]
assert [...y, x] == [2, 3, 1]
{x; ...yz} = {x = 1; y = 2; z = 3}
assert x == 1 and yz == {y = 2; z = 3}
assert {x; ...yz} == {x = 1; y = 2; z = 3}
```

## 虚拟运算符

用户不能直接使用的运算符。

### ref(x: T) -> Ref T | T

返回对对象的不可变引用。

### ref!(x: T!) -> Ref!T! | T!

返回对可变对象的可变引用。