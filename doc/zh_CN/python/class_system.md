# Python 类系统（与 Erg 相比）

## 方法

方法即使参照前方也没有关系，这并不是因为使用了特别的技术，而是因为方法的实际存在被动态检查。（在 Erg 中静态检查方法的实际存在。为了参照前方，必须将函数设为常量。）


```python
>>> class C:
...   def f(self, x):
...       if x == 0: return 0
...       else: return self.g(x)
...   def g(self, x): return self.f(x - 1)
```

## 继承，覆盖

被覆盖的某个方法 m 仅仅像变量的再代入那样被覆盖，参照母类 m 的方法在子类中参照被覆盖的 m。


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

因此，即使明显错误地被覆盖，在运行时也不会出现错误。


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

在 Erg 中静态检查与母类的一致性。在覆盖时，必须赋予装饰器，并且要覆盖的函数类型必须是要覆盖的函数类型的部分类型。


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
.f(self)は既にCで定義されています。オーバーライドするためには`Override`デコレータを付与し、`Self.() -> Nat`型かそのサブタイプである必要があります。
```

## 类型检查

类型检查大体上只限于函数自变量的类型检查。在 Python 中，大部分的操作都是方法调用。调用时，如果对象所属的类中附有方法的话，就到此为止。


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
