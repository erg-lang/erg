# 类型扩展（Type Widening）

例如定义如下的多相关数。


```erg
ids|T|(x: T, y: T) = x, y
```

代入相同类的实例对没有任何问题。如果代入包含关系中的其他类的实例对的话，就会被上播到较大的一方，成为相同的类型。另外，如果代入不在包含关系中的其他类，就会出现错误，这也很容易理解。


```erg
assert ids(1, 2) == (1, 2)
assert ids(1, 2.0) == (1.0, 2.0)
ids(1, "a") # TypeError
```

那么，拥有其他结构型的型的情况又会怎样呢？


```erg
i: Int or Str
j: Int or NoneType
ids(i, j) # ?
```

在解释这一点之前，我们必须注意一个事实，即 Erg 类型系统实际上没有看到类（在运行时）。


```erg
1: {__valueclass_tag__ = Phantom Int}
2: {__valueclass_tag__ = Phantom Int}
2.0: {__valueclass_tag__ = Phantom Ratio}
"a": {__valueclass_tag__ = Phantom Str}
ids(1, 2): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Int} == {__valueclass_tag__ = Phantom Int}
ids(1, 2.0): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Ratio} == {__valueclass_tag__ = Phantom Ratio} # Int < Ratio
ids(1, "a"): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Str} == Never # TypeError
```

之所以没有看到类，是因为有时不能正确看到，这是因为在 Erg 中对象的类属于运行时信息。例如，型对象的类是<gtr=“9”/>或者<gtr=“10”/>，这是哪一个只有执行后才能知道。当然，<gtr=“11”/>型的对象的类是由<gtr=“12”/>确定的，这时从类型系统中也能看到<gtr=“13”/>的结构型<gtr=“14”/>。

现在，让我们回到另一个结构类型的例子。从结论上来说，上面的代码如果没有类型，就会成为 TypeError。但是，如果用类型注释进行类型扩大，编译就可以通过。


```erg
i: Int or Str
j: Int or NoneType
ids(i, j) # TypeError: types of i and j not matched
# hint: try type widening (e.g. ids<Int or Str or NoneType>)
ids<Int or Str or NoneType>(i, j) # OK
```

有以下可能性。

* ：<gtr=“17”/>或<gtr=“18”/>。
* ：<gtr=“20”/>或<gtr=“21”/>时。
* ：<gtr=“23”/>且<gtr=“24”/>时。

有以下可能性。

* ：<gtr=“27”/>或<gtr=“28”/>时。
* ：<gtr=“30”/>或<gtr=“31”/>。
* 不能简化（独立类型）：当<gtr=“33”/>且<gtr=“34”/>时。

## 子例程定义中的类型扩展

在 Erg 中，返回值类型不一致时默认为错误。


```erg
parse_to_int s: Str =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # return Int object
# TypeError: mismatch types of return values
#     3 | do parse_to_int::return error("not numeric")
#                                 └─ Error
#     4 | ...
#         └ Int
```

为了解决这一问题，必须将返回类型显式指定为 Or 类型。


```erg
parse_to_int(s: Str): Int or Error =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # return Int object
```

这是为了不让子程序的返回值类型无意中混入其他类型的设计。但是，当返回值类型的选项是或<gtr=“36”/>等具有包含关系的类型时，向较大的类型对齐。
