# 元组

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/13_tuple.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/13_tuple.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

元组类似于数组，但可以保存不同类型的对象
这样的集合称为不等集合。相比之下，同构集合包括数组、集合等

```python
t = (1, True, "a")
(i, b, s) = t
assert(i == 1 and b == True and s == "a")
```

元组`t`可以以`t.n`的形式检索第n个元素； 请注意，与 Python 不同，它不是 `t[n]`
这是因为访问元组元素更像是一个属性(在编译时检查元素的存在，并且类型可以根据 `n` 改变)而不是方法(数组的 `[]` 是一种方法)

```python
assert t.0 == 1
assert t.1 == True
assert t.2 == "a"
```

括号 `()` 在不嵌套时是可选的

```python
t = 1, True, "a"
i, b, s = t
```

元组可以保存不同类型的对象，因此它们不能像数组一样被迭代

```python,compile_fail
t: ({1}, {2}, {3}) = (1, 2, 3)
(1, 2, 3).iter().map(x -> x + 1) # 类型错误: 类型 ({1}, {2}, {3}) 没有方法 `.iter()`
```

```python
# 如果所有类型都相同，则可以像数组一样用`(T; n)`表示，但这仍然不允许迭代
t: (Int; 3) = (1, 2, 3)
assert (Int; 3) == (Int, Int, Int)
```

但是，非同质集合(如元组)可以通过向上转换、相交等方式转换为同质集合(如数组)
这称为均衡

```python
(Int, Bool, Str) can be [T; 3] where T :> Int, T :> Bool, T :> Str
```

```python
t: (Int, Bool, Str) = (1, True, "a") # 非同质
a: [Int or Bool or Str; 3] = [1, True, "a"] # 同质的
_a: [Show; 3] = [1, True, "a"] # 同质的
_a.iter().map(x -> log x) # OK
t.try_into([Show; 3])? .iter().map(x -> log x) # OK
```

## 单元

零元素的元组称为 __unit__。一个单元是一个值，但也指它自己的类型

```python
unit = ()
(): ()
```

Unit是所有图元的超级类

```python
() > (Int; 0)
() > (Str; 0)
() :> (Int, Str)
...
```

该对象的用途是用于没有参数和没有返回值的过程等。Erg 子例程必须有参数和返回值。但是，在某些情况下，例如过程，可能没有有意义的参数或返回值，只有副作用。在这种情况下，我们将单位用作"无意义的正式值"

```python
p!() =.
    # `print!` does not return a meaningful value
    print! "Hello, world!"
p!: () => () # The parameter part is part of the syntax, not a tuple
```

但是，在这种情况下，Python 倾向于使用"无"而不是单位
在 Erg 中，当您从一开始就确定操作不会返回有意义的值(例如在过程中)时，您应该使用 `()`，并且当操作可能失败并且您可能会返回 `None` 将一无所获，例如在检索元素时

<p align='center'>
    <a href='./12_container_ownership.md'>上一页</a> | <a href='./14_record.md'>下一页</a>
</p>
