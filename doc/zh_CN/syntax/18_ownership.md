# 所有权制度

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/18_ownership.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/18_ownership.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

由于 Erg 是一种使用 Python 作为宿主语言的语言，因此内存管理的方法取决于 Python 的实现。
但语义上 Erg 的内存管理与 Python 的不同。 一个显着的区别在于所有权制度和禁止循环引用。

## 所有权

Erg 有一个受 Rust 启发的所有权系统。
Rust 的所有权系统通常被认为是深奥的，但 Erg 的所有权系统被简化为直观。
在 Erg 中，__mutable objects__ 是拥有的，并且在所有权丢失后无法引用。

```python
v = [1, 2, 3].into [Int; !3]

push! vec, x =
    vec.push!(x)
    vec

# v ([1, 2, 3])的内容归w所有
w = push! v, 4
print! v # 错误：v 被移动了
print!w # [1, 2, 3, 4]
```

例如，当一个对象被传递给一个子程序时，就会发生所有权转移。
如果您想在赠送后仍然拥有所有权，则需要克隆、冻结或借用。
但是，如后所述，可以借用的情况有限。

## 复制

复制一个对象并转移其所有权。 它通过将 `.clone` 方法应用于实际参数来做到这一点。
复制的对象与原始对象完全相同，但相互独立，不受更改影响。

复制相当于 Python 的深拷贝，由于它完全重新创建相同的对象，因此计算和内存成本通常高于冻结和借用。
需要复制对象的子例程被称为"参数消耗"子例程。

```python
capitalize s: Str!=
    s. capitalize!()
    s

s1 = !"hello"
s2 = capitalize s1.clone()
log s2, s1 # !"HELLO hello"
```

## 冻结

我们利用了不可变对象可以从多个位置引用的事实，并将可变对象转换为不可变对象。
这称为冻结。 例如，在从可变数组创建迭代器时会使用冻结。
由于您不能直接从可变数组创建迭代器，请将其转换为不可变数组。
如果您不想破坏数组，请使用 [`.freeze_map` 方法](./type/18_mut.md)。

```python
# 计算迭代器产生的值的总和
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3]
x.push!(4)
i = x.iter() # 类型错误：[Int; !4] 没有方法 `iter`
y = x.freeze()
i = y.iter()
assert sum(i) == 10
y # y 仍然可以被触摸
```

## 借

借用比复制或冻结便宜。
可以在以下简单情况下进行借款：

```python
peek_str ref(s: Str!) =
    log s

s = !"hello"
peek_str s
```

借来的值称为原始对象的 __reference__。
您可以"转租"对另一个子例程的引用，但您不能使用它，因为您只是借用它。

```python
steal_str ref(s: Str!) =
    # 由于日志函数只借用参数，所以可以转租
    log s
    # 错误，因为丢弃函数消耗了参数
    discard s # OwnershipError: 不能消费借来的值
    # 提示：使用 `clone` 方法
```

```python
steal_str ref(s: Str!) =
    # 这也不好(=消耗右边)
     x = s # OwnershipError: 不能消费借来的值
    x
```

Erg 的引用比 Rust 的更严格。 引用是语言中的一等对象，但不能显式创建，它们只能指定为通过 `ref`/`ref!` 传递的参数。
这意味着您不能将引用填充到数组中或创建将引用作为属性的类。

但是，这样的限制是语言中的自然规范，一开始就没有引用，而且它们并没有那么不方便。

## 循环引用

Erg 旨在防止无意的内存泄漏，如果内存检查器检测到循环引用，则会发出错误。 在大多数情况下，这个错误可以通过弱引用 `Weak` 来解决。 但是，由于无法生成循环图等具有循环结构的对象，因此我们计划实现一个 API，可以将循环引用作为不安全操作生成。

<p align='center'>
    <a href='./17_mutability.md'>上一页</a> | <a href='./19_visibility.md'>下一页</a>
</p>
