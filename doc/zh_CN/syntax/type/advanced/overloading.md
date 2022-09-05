# 重载

Erg 不支持 __ad hoc 多态性__。 也就是说，函数和种类（重载）的多重定义是不可能的。 但是，您可以通过使用特征和补丁的组合来重现重载行为。
您可以使用特征而不是特征类，但随后将涵盖所有实现 `.add1` 的类型。

```python
Add1 = Trait {
    .add1: Self.() -> Self
}
IntAdd1 = Patch Int, Impl := Add1
IntAdd1.
    add1 self = self + 1
RatioAdd1 = Patch Ratio, Impl := Add1
RatioAdd1.
    add1 self = self + 1.0

add1|X <: Add1| x: X = x.add1()
assert add1(1) == 2
assert add1(1.0) == 2.0
```

这种接受一个类型的所有子类型的多态称为__subtyping polymorphism__。

如果每种类型的过程完全相同，则可以编写如下。 当行为从类到类（但返回类型相同）时，使用上述内容。
使用类型参数的多态称为 __parametric polymorphism__。 参数多态性通常与子类型结合使用，如下所示，在这种情况下，它是参数和子类型多态性的组合。

```python
add1|T <: Int or Str| x: T = x + 1
assert add1(1) == 2
assert add1(1.0) == 2.0
```

此外，可以使用默认参数重现具有不同数量参数的类型的重载。

```python
C = Class {.x = Int; .y = Int}
C.
    new(x, y := 0) = Self::__new__ {.x; .y}

assert C.new(0, 0) == C.new(0)
```

Erg 的立场是，您不能定义行为完全不同的函数，例如根据参数的数量具有不同的类型，但如果行为不同，则应该以不同的方式命名。

综上所述，Erg 禁止重载，采用子类型加参数多态，原因如下。

首先，重载函数分布在它们的定义中。 这使得在发生错误时很难报告错误的原因。
此外，导入子程序可能会改变已定义子程序的行为。

```python
{id; ...} = import "foo"
...
id x: Int = x
...
id x: Ratio = x
...
id "str" # 类型错误：没有为 Str 实现 id
# 但是……但是……这个错误是从哪里来的？
```

其次，它与默认参数不兼容。 当具有默认参数的函数被重载时，会出现一个优先级的问题。

```python
f x: Int = ...
f(x: Int, y := 0) = ...

f(1) # 选择哪个？
```

此外，它与声明不兼容。
声明 `f: Num -> Num` 不能指定它引用的定义。 这是因为 `Int -> Ratio` 和 `Ratio -> Int` 不包含在内。

```python
f: Num -> Num
f(x: Int): Ratio = ...
f(x: Ratio): Int = ...
```

并且语法不一致：Erg禁止变量重新赋值，但是重载的语法看起来像重新赋值。
也不能用匿名函数代替。

```python
# 同 `f = x -> body`
f x = body

# 一样……什么？
f x: Int = x
f x: Ratio = x
```
