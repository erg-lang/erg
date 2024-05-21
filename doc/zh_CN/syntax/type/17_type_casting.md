# 类型转换

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/17_type_casting.md%26commit_hash%3D7d7849b4932909197c185c1737dcc1f63cce701c)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/17_type_casting.md&commit_hash=7d7849b4932909197c185c1737dcc1f63cce701c)

## 向上转换

因为 Python 是一种使用鸭子类型的语言，所以没有强制转换的概念。没有必要向上转换，本质上也没有向下转换
但是，Erg 是静态类型的，因此有时必须进行强制转换
一个简单的例子是 `1 + 2.0`: `+`(Int, Ratio) 或 Int(<: Add(Ratio, Ratio)) 操作在 Erg 语言规范中没有定义。这是因为 `Int <: Ratio`，所以 1 向上转换为 1.0，即 Ratio 的一个实例

~~Erg扩展字节码在BINARY_ADD中增加了类型信息，此时类型信息为Ratio-Ratio。在这种情况下，BINARY_ADD 指令执行 Int 的转换，因此没有插入指定转换的特殊指令。因此，例如，即使您在子类中重写了某个方法，如果您将父类指定为类型，则会执行类型强制，并在父类的方法中执行该方法(在编译时执行名称修改以引用父母的方法)。编译器只执行类型强制验证和名称修改。运行时不强制转换对象(当前。可以实现强制转换指令以优化执行)。~~

```python
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    # Override 需要 Override 装饰器
    @Override
    greet!() = print! "Hello from Child"

greet! p: Parent = p.greet!()

parent = Parent.new()
child = Child.new()

parent # 来自Parent的问候！
child #  来自child的问候！
```

此行为不会造成与 Python 的不兼容。首先，Python 没有指定变量的类型，所以可以这么说，所有的变量都是类型变量。由于类型变量会选择它们可以适应的最小类型，因此如果您没有在 Erg 中指定类型，则可以实现与 Python 中相同的行为

```python
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    greet!() = print! "Hello from Child" Child.

greet! some = some.greet!()

parent = Parent.new()
child = Child.new()

parent # 来自Parent的问候！
child  # 来自child的问候！
```

您还可以使用 `.from` 和 `.into`，它们会为相互继承的类型自动实现

```python
assert 1 == 1.0
assert Ratio.from(1) == 1.0
assert 1.into<Ratio>() == 1.0
```

## Forced upcasting

In many cases, upcasting of objects is automatic, depending on the function or operator that is called.
However, there are cases when you want to force upcasting. In that case, you can use `as`.

```python,compile_fail
n = 1
n.times! do: print!
    print! "Hello"

i = n as Int
i.times! do: # ERR
    "Hello"

s = n as Str # ERR
```

You cannot cast to unrelated types or subtypes with ``as``.

## Forced casting

You can use `typing.cast` to force casting. This can convert the target to any type.
In Python, `typing.cast` does nothing at runtime, but in Erg the conversion will be performed by the constructor if object's type is built-in[<sup id="f1">1</sup>](#1).
For non-built-in types, the safety is not guaranteed at all.

```python
typing = pyimport "typing"

C = Class { .x = Int }

s = typing.cast Str, 1

assert s == "1"
print! s + "a" # 1a

c = typing.cast C, 1
print! c.x # AttributeError: 'int' object has no attribute 'x'
```

## 向下转换

由于向下转换通常是不安全的并且转换方法很重要，我们改为实现`TryFrom.try_from`

```python
IntTryFromFloat = Patch Int
IntTryFromFloat.
    try_from r: Float =
        if r.ceil() == r:
            then: r.ceil()
            else: Error "conversion failed".
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> This conversion is a byproduct of the current implementation and will be removed in the future. [↩](#f1) </span>

<p align='center'>
    <a href='./16_subtyping.md'>上一页</a> | <a href='./18_mut.md'>下一页</a>
</p>