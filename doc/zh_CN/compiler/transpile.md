# Erg 代码如何转译成 Python 代码?

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/transpile.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/transpile.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

准确地说，Erg 代码被转译为 Python 字节码
但是，由于 Python 字节码几乎可以重构为 Python 代码，因此这里以等效的 Python 代码为例
顺便说一句，这里展示的示例是低优化级别
更高级的优化消除了不需要实例化的东西

## 记录，记录类型

它将被转译为一个命名元组
对于 namedtuple，请参阅 [此处](https://docs.python.jp/3/library/collections.html# collections.namedtuple)
有一个类似的函数，dataclass，但是由于 `__eq__` 和 `__hash__` 的自动实现，dataclass 的性能略有下降

```python
Employee = Class {.name = Str; .id = Int}

employee = Employee.new({.name = "John Smith"; .id = 100})

assert employee.name == "John Smith"
```

```python
from typing import NamedTuple

class Employee(NamedTuple):
    __records__ = ['name', 'id']
    name: str
    id: int

employee = Employee('John Smith', 100)

assert employee.name == 'John Smith'
```

如果可以进一步优化，它也将转换为简单的元组

## 多态类型

> 在制品

## 即时范围

如果没有发生命名空间冲突，它只会被破坏和扩展
`x::y` 等名称在字节码中使用，不能与 Python 代码关联，但如果强制表示，则会如下所示

```python
x =
    y = 1
    y+1
```

```python
x::y = 1
x = x::y + 1
```

万一发生冲突，定义和使用只能在内部引用的函数

```python
x =
    y = 1
    y+1
```

```python
def _():
    x=1
    y = x
    return y + 1
x = _()
```

## 可见性

它对公共变量没有任何作用，因为它是 Python 的默认值
私有变量由 mangling 处理

```python
x=1
y =
    x = 2
    assert module::x == 2
```

```python
module::x = 1
y::x = 2
assert module::x == 2
y = None
```