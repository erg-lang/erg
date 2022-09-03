# Erg 代码如何转堆到 Python 代码中？

准确地说，Erg 代码被转堆为 Python 字节代码。但是 Python 字节码几乎可以恢复为 Python 代码，所以这里给出一个等效的 Python 代码作为例子。顺便说一下，这里的示例是优化级别较低的示例。进一步的高级优化将清除不需要生成实体的内容。

## Record, Record type

变换成 namedtuple。有关 namedtuple 的信息，请参见。类似的功能包括 dataclass，但由于自动实现和<gtr=“10”/>，dataclass 的性能略有下降。


```erg
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

如果可以进一步优化，它还将转换为简单的元组。

## Polymorphic Type

> WIP

## Instant Scope

如果名称空间中不发生冲突，则只会进行弯曲和展开。像这样的名称在字节码中使用，不能与 Python 代码相对应，但如果强行表示，则会出现以下情况。


```erg
x =
    y = 1
    y + 1
```


```python
x::y = 1
x = x::y + 1
```

如果发生冲突，请定义和使用只能在内部引用的函数。


```erg
x =
    y = 1
    y + 1
```


```python
def _():
    x = 1
    y = x
    return y + 1
x = _()
```

## Visibility

对于公共变量，它是 Python 的缺省值，因此不执行任何操作。私域变量是由 Munging 处理的。


```erg
x = 1
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
