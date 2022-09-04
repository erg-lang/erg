# Python 類系統（與 Erg 相比）

## 方法

方法即使參照前方也沒有關係，這並不是因為使用了特別的技術，而是因為方法的實際存在被動態檢查。 （在 Erg 中靜態檢查方法的實際存在。為了參照前方，必須將函數設為常量。）


```python
>>> class C:
...   def f(self, x):
...       if x == 0: return 0
...       else: return self.g(x)
...   def g(self, x): return self.f(x - 1)
```

## 繼承，覆蓋

被覆蓋的某個方法 m 僅僅像變量的再代入那樣被覆蓋，參照母類 m 的方法在子類中參照被覆蓋的 m。


```python
>>> class C:
...   def f(self): return 1
...   def g(self): return self.f()
...
>>> class D(C):
...   def f(self): return 2
...
>>> D().g()
2
```

因此，即使明顯錯誤地被覆蓋，在運行時也不會出現錯誤。


```python
>>> class C:
...   def f(self): return 1
...   def g(self): return self.f() + 1
...
>>> class D(C):
...   def f(self): return "a"
...
>>> D().g()
Traceback (most recent call last):
  File "<stdin>", line 1, in <module>
  File "<stdin>", line 3, in g
TypeError: can only concatenate str (not "int") to str
```

在 Erg 中靜態檢查與母類的一致性。在覆蓋時，必須賦予裝飾器，並且要覆蓋的函數類型必須是要覆蓋的函數類型的部分類型。


```erg
>>> C = Class()
...   .f self = 1
...   .g self = self.f() + 1
...
>>> D = Inherit C
...   .f self = "a"
...
Error[#XX]: File "<stdin>", line 5, in D
.f(self) is already defined in C. To override f, it must be added `Override` decorator and its type must be `Self.() -> Nat` or the subtype of that
.f(self)は既にCで定義されています。オーバーライドするためには`Override`デコレータを付與し、`Self.() -> Nat`型かそのサブタイプである必要があります。
```

## 類型檢查

類型檢查大體上只限於函數自變量的類型檢查。在 Python 中，大部分的操作都是方法調用。調用時，如果對象所屬的類中附有方法的話，就到此為止。


```python
def f(x):
    return x.m()

class C:
    def m(self): return None

c = C()
f(c)
f(1) # TypeError
```


```erg
# f: |T, X <: {.m = Self.() -> T}| X -> T
f(x) = x.m()

C = Class()
C.m(self) = None

c = C.new()
f(c)
f(1) # TypeError: f takes a type has method `.m` as an argument, but passed Nat
```