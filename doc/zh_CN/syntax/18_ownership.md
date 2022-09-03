# 所有权制度

由于 Erg 是以 Python 为主机语言的语言，因此管理内存的方式依赖于 Python 的处理系统。然而，从语义上讲，Erg 的内存管理与 Python 的内存管理不同。显著的区别体现在所有权制度和禁止循环引用。

## 所有权

Erg 有一个所有权系统受到拉斯特的影响。Rust 的所有权系统通常被称为晦涩难懂，但 Erg 的所有权系统被简化为直观。Erg 拥有的所有权，一旦失去所有权，就无法查看该对象。


```erg
v = [1, 2, 3].into [Int; !3]

push! vec, x =
    vec.push!(x)
    vec

# vの中身([1, 2, 3])の所有権はwに移る
w = push! v, 4
print! v # error: v was moved
print! w # [1, 2, 3, 4]
```

例如，在将对象传递到子例程时会发生所有权移动。如果你希望在传递后仍拥有所有权，则必须复制（cloning）、冻结（freeze）或借用（borrowing）。但是，如下文所述，可以借用的场合有限。

## 复制

复制对象并转移其所有权。通过将方法应用于实际参数来完成此操作。复制的对象与原始对象完全相同，但它们彼此独立，不受更改的影响。

复制相当于 Python 的深度副本，因为要重新创建整个相同的对象，所以与冻结和借用相比，通常计算和内存成本更高。需要复制对象的子例程称为使用参数子例程。


```erg
capitalize s: Str! =
    s.capitalize!()
    s

s1 = !"hello"
s2 = capitalize s1.clone()
log s2, s1 # !"HELLO hello"
```

## 冻结

利用可变对象可以从多个位置引用，将可变对象转换为不变对象。这叫冻结。冻结可用于创建可变阵列的迭代器。变量数组无法直接创建迭代器，因此将其转换为不变数组。如果不想破坏数组，请使用 [方法] （./type/mut.md）等。


```erg
# イテレータが出す値の合計を計算する
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3]
x.push!(4)
i = x.iter() # TypeError: [Int; !4] has no method `iter`
y = x.freeze()
i = y.iter()
assert sum(i) == 10
y # この後もyは触れられる
```

## 借用

借用比复制和冻结成本更低。在以下简单情况下，可以借用。


```erg
peek_str ref(s: Str!) =
    log s

s = !"hello"
peek_str s
```

对于原始对象，借用的值称为。你可以将引用“转借”给另一个子例程，但不能消费，因为它只是借用。


```erg
steal_str ref(s: Str!) =
    # log関数は引数を借用するだけなので、又貸しできる
    log s
    # discard関数は引数を消費するので、エラー
    discard s # OwnershipError: cannot consume a borrowed value
    # hint: use `clone` method
```


```erg
steal_str ref(s: Str!) =
    # これもダメ(=は右辺を消費する)
    x = s # OwnershipError: cannot consume a borrowed value
    x
```

Erg 引用比 Rust 具有更强的约束。引用是第一级语言对象，但不能显式生成，只能通过/<gtr=“12”/>指定实际参数的传递方式。这意味着你不能将引用合并到数组中，也不能创建以引用为属性的类。

尽管如此，这种限制在没有参照的语言中本来就是理所当然的规范，并没有那么不方便。

## 循环引用

Erg 的设计目的是防止意外发生内存泄漏，当内存检查器检测到循环引用时，它会发出错误消息。在大多数情况下，可以使用弱引用来解决此错误。但是，由于这无法生成具有循环结构的对象（如循环图），因此我们计划实现一个 API，该 API 可以生成循环引用作为 unsafe 操作。

<p align='center'>
    <a href='./17_mutability.md'>Previous</a> | <a href='./19_visibility.md'>Next</a>
</p>
