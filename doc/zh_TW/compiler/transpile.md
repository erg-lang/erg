# Erg 代碼如何轉堆到 Python 代碼中？

準確地說，Erg 代碼被轉堆為 Python 字節代碼。但是 Python 字節碼幾乎可以恢復為 Python 代碼，所以這裡給出一個等效的 Python 代碼作為例子。順便說一下，這裡的示例是優化級別較低的示例。進一步的高級優化將清除不需要生成實體的內容。

## Record, Record type

變換成 namedtuple。有關 namedtuple 的信息，請參見。類似的功能包括 dataclass，但由於自動實現和<gtr=“10”/>，dataclass 的性能略有下降。


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

如果可以進一步優化，它還將轉換為簡單的元組。

## Polymorphic Type

> WIP

## Instant Scope

如果名稱空間中不發生衝突，則只會進行彎曲和展開。像這樣的名稱在字節碼中使用，不能與 Python 代碼相對應，但如果強行表示，則會出現以下情況。


```erg
x =
    y = 1
    y + 1
```


```python
x::y = 1
x = x::y + 1
```

如果發生衝突，請定義和使用只能在內部引用的函數。


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

對於公共變量，它是 Python 的缺省值，因此不執行任何操作。私域變量是由 Munging 處理的。


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