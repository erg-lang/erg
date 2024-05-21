# 可变类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/18_mut.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/18_mut.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

> __Warning__: 本节中的信息是旧的并且包含一些错误

默认情况下，Erg 中的所有类型都是不可变的，即它们的内部状态无法更新
但是你当然也可以定义可变类型。变量类型用 `!` 声明

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is \{self::name}. I am \{self::age}."
    inc_age!ref!self = self::name.update!old -> old + 1
```

准确地说，基类型是可变类型或包含可变类型的复合类型的类型必须在类型名称的末尾有一个"！"。没有 `!` 的类型可以存在于同一个命名空间中，并被视为单独的类型
在上面的例子中，`.age` 属性是可变的，`.name` 属性是不可变的。如果即使一个属性是可变的，那么整个属性也是可变的

可变类型可以定义重写实例的过程方法，但具有过程方法并不一定使它们可变。例如数组类型`[T; N]` 实现了一个 `sample!` 随机选择一个元素的方法，但当然不会破坏性地修改数组

对可变对象的破坏性操作主要是通过 .update! 方法完成的。`.update!` 方法是一个高阶过程，它通过应用函数 `f` 来更新 `self`

```python
i = !1
i.update! old -> old + 1
assert i == 2
```

`.set!` 方法只是丢弃旧内容并用新值替换它。.set!x = .update!_ -> x

```python
i = !1
i.set! 2
assert i == 2
```

`.freeze_map` 方法对不变的值进行操作

```python
a = [1, 2, 3].into [Nat; !3]
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(List)
```

在多态不可变类型中，该类型的类型参数"T"被隐式假定为不可变

```python
# ImmutType < Type
KT: ImmutType = Class ...
K!T: Type = Class ...
```

在标准库中，变量 `(...)!` 类型通常基于不可变 `(...)` 类型。但是，`T!` 和 `T` 类型没有特殊的语言关系，并且不能这样构造 [<sup id="f1">1</sup>](#1)

From the above explanation, mutable types include not only those that are themselves mutable, but also those whose internal types are mutable.
Types such as `{x: Int!}` and `[Int!; 3]` are internal mutable types where the object inside is mutable and the instance itself is not mutable.

## Cell! T

Mutable types are already available for `Int` and arrays, but how can we create mutable types for general immutable types? For example, in the case of `{x = Int; y = Int}`, corresponding mutable type is `{x = Int!; y = Int!}`, etc. But how did `Int!` made from `Int`?

Erg provides `Cell!` type for such cases.
This type is like a box for storing immutable types. This corresponds to what is called a reference (ref) in ML and other languages.

```python
IntOrStr = Inr or Str
IntOrStr! = Cell! IntOrStr
x = IntOrStr!.new 1
assert x is! 1 # `Int or Str` cannot compare with `Int` directly, so use `is!` (this compares object IDs) instead of `==`.
x.set! "a"
assert x is! "a"
```

An important property is that `Cell! T` is a subtype of `T`. Therefore, an object of type `Cell! T` can use all the methods of type `T`.

```python
# definition of `Int!`
Int! = Cell! Int
...

i = !1
assert i == 1 # `i` is casted to `Int`
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> It is intentional that `T!` and `T` types have no special linguistic relationship. It's a design. If there is a relationship, for example, if the `T`/`T!` type exists in the namespace, it will not be possible to introduce the `T!`/`T` type from another module. Also, the mutable type is not uniquely defined for the immutable type. Given the definition `T = (U, V)`, the possible variable subtypes of `T!` are `(U!, V)` and `(U, V!)`. [↩](#f1)</span>

<p align='center'>
    <a href='./17'>上一页</a> | <a href='./19_bound.md'>下一页</a>
</p>
