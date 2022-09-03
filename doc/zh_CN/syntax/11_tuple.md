# 元

元组类似于数组，但可以包含不同类型的对象。这样的收藏称为非等质收藏。与此相对，等质集合包括数组和集合。


```erg
t = (1, True, "a")
(i, b, s) = t
assert(i == 1 and b == True and s == "a")
```

元组可以以<gtr=“14”/>的形式检索第 n 个元素。请注意，与 Python 不同，它不是。这是因为元组元素的访问更接近于属性，而不是方法（数组中的<gtr=“16”/>是方法）（编译时检查元素是否存在，类型可以根据 n 而变化）。


```erg
assert t.0 == 1
assert t.1 == True
assert t.2 == "a"
```

不嵌套时，括号是可选的。


```erg
t = 1, True, "a"
i, b, s = t
```

元组可以包含不同类型的对象，但不能包含数组之类的小版本。


```erg
t: ({1}, {2}, {3}) = (1, 2, 3)
(1, 2, 3).iter().map(x -> x + 1) # TypeError: type ({1}, {2}, {3}) has no method `.iter()`
# すべて同じ型の場合配列と同じように`(T; n)`で表せるが、これでもイテレーションは出来ない
t: (Int; 3) = (1, 2, 3)
assert (Int; 3) == (Int, Int, Int)
```

但是，非等质集合（如元组）可以通过上传、Intersection 等转换为等质集合（如数组）。这叫做等质化。


```erg
(Int, Bool, Str) can be [T; 3] | T :> Int, T :> Bool, T :> Str
```


```erg
t: (Int, Bool, Str) = (1, True, "a") # non-homogenous
a: [Int or Bool or Str; 3] = [1, True, "a"] # homogenous
_a: [Show; 3] = [1, True, "a"] # homogenous
_a.iter().map(x -> log x) # OK
t.try_into([Show; 3])?.iter().map(x -> log x) # OK
```

## 单位

具有 0 个元素的元组称为单元。单位是一个值，但也指其类型本身。


```erg
unit = ()
(): ()
```

单元是所有元素 0 元组的超类。


```erg
() > (Int; 0)
() > (Str; 0)
```

此对象的用途包括参数、没有返回值的过程等。Erg 子例程必须具有参数和返回值。但是，在某些情况下，例如在过程中，可能会产生副作用，但没有有意义的参数返回值。在这种情况下，单位作为“没有意义的，形式上的值”来使用。


```erg
# ↓ 実はこの括弧はユニット
p!() =
    # `print!`は意味のある値を返さない
    print! "Hello, world!"
p!: () => ()
```

但是，Python 在这种情况下更倾向于使用而不是单位。在 Erg 中，如果一开始就确定不返回有意义的值（如过程），则返回<gtr=“19”/>；如果操作失败，可能一无所获（如元素检索），则返回<gtr=“20”/>。

## 参数和元组

实际上，Erg 的所有对象都是一个参数，一个返回值。具有 N 个参数的子程序仅接受“一个具有 N 个元素的元组”作为参数。


```erg
# f x = ...は暗黙にf(x) = ...とみなされる
f x = x
assert f(1) == 1
f(1, 2, 3) # ArgumentError: f takes 1 positional argument but 3 were given
# 可変個の引数を受け取る
g x: Int, ...y: Int = y
assert (2, 3) == g 1, 2, 3
```

这将解释函数的类型。


```erg
assert f in T: {(T,) -> T | T}
assert g in {(Int, ...(Int; N)) -> (Int; N) | N: Nat}
```

准确地说，函数的输入不是元组，而是具有默认属性的命名元组。这是一个特殊的元组，只能作为函数的参数使用，可以像记录一样命名并具有缺省值。


```erg
f(x: Int, y=0) = x + y
f: (Int, y=Int) -> Int

f(x=0, y=1)
f(y=1, x=0)
f(x=0)
f(0)
```

<p align='center'>
    <a href='./10_array.md'>Previous</a> | <a href='./12_dict.md'>Next</a>
</p>
