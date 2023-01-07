# 类型擦除

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/erasure.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/erasure.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

类型擦除是将类型参数设置为 `_` 并故意丢弃其信息的过程。类型擦除是许多多态语言的特性，但在 Erg 的语法上下文中，将其称为类型参数擦除更为准确

类型擦除的最常见示例是 `[T, _]`。数组在编译时并不总是知道它们的长度。例如，引用命令行参数的 `sys.argv` 的类型为 `[Str, _]`。由于 Erg 的编译器无法知道命令行参数的长度，因此必须放弃有关其长度的信息
然而，一个已经被类型擦除的类型变成了一个未被擦除的类型的父类型(例如`[T; N] <: [T; _]`)，所以它可以接受更多的对象
类型的对象`[T; N]` 当然可以使用 `[T; _]`，但使用后会删除`N`信息。如果长度没有改变，那么可以使用`[T; N]` 在签名中。如果长度保持不变，则必须由签名指示

```python
# 保证不改变数组长度的函数(例如，排序)
f: [T; N] -> [T; N] # 没有的函数 (f: [T; N])
# 没有的功能(例如过滤器)
g: [T; n] -> [T; _]
```

如果您在类型规范本身中使用 `_`，则类型将向上转换为 `Object`
对于非类型类型参数(Int、Bool 等)，带有 `_` 的参数将是未定义的

```python
i: _ # i: Object
[_; _] == [Object; _] == Array
```

类型擦除与省略类型说明不同。一旦类型参数信息被删除，除非您再次声明它，否则它不会被返回

```python
implicit = (1..5).iter().map(i -> i * 2).to_arr()
explicit = (1..5).iter().map(i -> i * 2).into(Array(Nat))
```

在 Rust 中，这对应于以下代码:

```rust
let partial = (1..6).iter().map(|i| i * 2).collect::<Vec<_>>();
```

Erg 不允许部分省略类型，而是使用高阶种类多态性

```python
# collect 是采用 Kind 的高阶 Kind 方法
hk = (1..5).iter().map(i -> i * 2).collect(Array)
hk: Array(Int)
```
