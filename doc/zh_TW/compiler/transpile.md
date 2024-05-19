# Erg 代碼如何轉譯成 Python 代碼?

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/transpile.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/transpile.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

準確地說，Erg 代碼是被轉譯為 Python 字節碼。鑒于 Python 字節碼幾乎可以被重構為 Python 文本代碼，因此這里以等效的 Python 代碼為例。
順便說一下，這里展示的示例是低優化級別。更高級的優化會消除不需要實例化的東西

## 記錄，記錄類型

它將被轉換為一個命名元組（namedtuple）。
對于 namedtuple，請參閱 [此處](https://docs.python.org/zh-cn/3/library/collections.html#collections.namedtuple)。
有一個類似的功能，數據類（dataclass），但由于__eq__和__hash__的自動實現，數據類在性能上略有下降

```python
employee = Employee.new({.name = "John Smith"; .id = 100})
assert employee.name == "John Smith"
```


```python
employee = NamedTuple(['name', 'id'])('John Smith', 100)
assert employee.name == 'John Smith'
```

如果可以進一步優化，它還將被轉換為簡單的元組。

## 多態類型

> 在制品

## 即時范圍

如果沒有發生命名空間沖突，它只會被破壞和擴展
`x::y` 等名稱在字節碼中使用，不能與 Python 代碼關聯，但如果強制表示，則會如下所示

```python
x =
    y = 1
    y+1
```

```python
x::y = 1
x = x::y + 1
```

萬一發生沖突，定義和使用只能在內部引用的函數

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

## 可見性

它對公共變量沒有任何作用，因為它是 Python 的默認值
私有變量由 mangling 處理

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

## Patch

```python
func b: Bool =
    Invert = Patch Bool
    Invert.
        invert self = not self
    b.invert()
```

```python
def func(b):
    def Invert::invert(self): return not self
    return Invert::invert(b)
```