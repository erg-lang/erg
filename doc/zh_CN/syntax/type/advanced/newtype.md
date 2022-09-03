# Newtype pattern

下面是 Rust 常用的 newtype 模式的 Erg 版本。

Erg 可以按如下方式定义类型别名，但仅指同一类型。


```erg
UserId = Int
```

因此，例如，即使类型的数值是 8 位正数，因为它与<gtr=“4”/>类型相同，所以可以输入 10 或-1. 如果，-1 是可以弹的，但是 8 位数的性质仅用 Erg 的类型系统是不能表现的。

再比如设计某个数据库的系统时，有几类 ID。随着 ID 类型的增加，例如用户 ID，商品 ID，订单 ID 等，可能会出现错误，即向函数传递不同类型的 ID。用户 ID 和商品 ID 等即使在结构上等价，在语义上也是不同的。

newtype 模式是这种情况下的理想设计模式。


```erg
UserId = Class {id = Nat}
UserId.
    new id: Nat =
        assert id.dights().len() == 8, else: "UserId must be a positive number with length 8"
        UserId::__new__ {id;}

i = UserId.new(10000000)
print! i # <__main__.UserId object>
i + UserId.new(10000001) # TypeError: + is not implemented between `UserId` and `UserId`
```

构造函数保证了 8 位数的先决条件。由于丢失了<gtr=“7”/>的所有方法，因此必须重新定义每次所需的运算。如果重新定义的成本不相称，最好使用继承。相反，你可能希望使用没有方法的特性，因此请根据具体情况选择适当的方法。
