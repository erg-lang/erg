# ErgコードはPythonコードにどのようにトランスパイルされるか？

正確には、ErgコードはPythonバイトコードにトランスパイルされます。
しかしPythonバイトコードはほぼPythonコードに復元できるので、ここでは等価なPythonコードを例として上げています。
ちなみに、ここで紹介する例は最適化レベルの低いものです。
さらに高度な最適化が施されると、実体を生成する必要のないものは消去されます。

## Record, Record type

namedtupleにトランスパイルされます。
namedtupleについては、[こちら](https://docs.python.jp/3/library/collections.html#collections.namedtuple)を参照してください。
似たような機能にdataclassがありますが、dataclassは`__eq__`や`__hash__`が自動実装されるなどの影響で少しパフォーマンスが落ちます。

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

また、更に最適化できる場合は単なるタプルに変換されます。

## Polymorphic Type

> WIP

## Instant Scope

名前空間内での衝突が起きない場合は、単にマングリングして展開されます。
`x::y`などの名前はバイトコードで使用されるものでPythonコードと対応させる事はできませんが、無理やり表現すると以下のようになります。

```erg
x =
    y = 1
    y + 1
```

```python
x::y = 1
x = x::y + 1
```

衝突する場合は、内部的にしか参照できない関数を定義して使用します。

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

公開変数に関してはPythonのデフォルトなので何もしません。
非公開変数はマングリングで対処しています。

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
