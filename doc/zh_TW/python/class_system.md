# Python 類系統(與 Erg 比較)

## 方法

方法可以被前向引用，但這不是一種特殊的技術。
這是因為動態檢查方法的存在。
(在 Erg 中，方法存在是靜態檢查的。對于前向引用，函數必須是常量。)

```python
>>> class C:
...   def f(self, x):
...       if x == 0: return 0
...       else: return self.g(x)
...   def g(self, x): return self.f(x - 1)
```

## 繼承，覆蓋

一些被覆蓋的方法 m 被簡單地覆蓋，就像變量重新分配一樣。
在父類中引用 m 的方法將在子類中引用被覆蓋的 m。

```python
>>> class C:
...   def f(self): return 1
...   def g(self): return self.f()
...
>>> class D(C):
...   def f(self): return 2
...
>>>> D().g()
2
```

因此，即使它顯然被錯誤地覆蓋，直到運行時才會出錯。

```python
>>>> class C:
...   def f(self): return 1
...   def g(self): return self.f() + 1
...
>>> class D(C):
...   def f(self): return "a"
...
>>> D().g()
Traceback (most recent call last):
  File "<stdin>", line 1, in <module
  File "<stdin>", line 3, in g
類型錯誤：只能將str(不是“int”)連接到str
```

Erg 靜態檢查與父類的一致性。
重寫時必須給出“Override”裝飾器，并且重寫函數的類型必須是被重寫函數類型的子類型。

```python
>>> C = Class()
...   .f self = 1
...   .g self = self.f() + 1
...
>>> D = Inherit C
...   .f self = "a"
...
錯誤[#XX]：文件“<stdin>”，第 5 行，在 D 中
要覆蓋 f，必須添加 `Override` 裝飾器，其類型必須是 `Self.() -> Nat` 或其子類型
f(self) 已在 C 中定義。要覆蓋 f，必須添加 `Override` 裝飾器，其類型必須為 `Self. 要覆蓋，必須給它一個 `Override` 裝飾器，并且它的類型必須是 `Self.() -> Nat` 或 that.f(self) 的子類型。
```

## 類型檢查

類型檢查通常都是關于檢查函數參數的類型。
在 Python 中，大多數操作都是方法調用。 如果對象所屬的類在調用時沒有附加方法，就是這樣。

```python
def f(x):
    return x.m()

class C:
    def m(self): return None

c = C()
f(c)
f(1) # 類型錯誤
```

```python
# f: |T, X <: {.m = Self.() -> T}| X -> T
f(x) = x.m()

C = Class()
C.m(self) = None

c = C.new()
f(c)
f(1) # 類型錯誤：f 將具有方法 `.m` 的類型作為參數，但傳遞了 Nat
```
