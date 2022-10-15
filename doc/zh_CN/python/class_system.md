# Python 类系统(与 Erg 比较)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/class_system.md%26commit_hash%3D2ecd249a2a99dc93dde2660b8d50bfa4fa0b03b9)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/class_system.md&commit_hash=2ecd249a2a99dc93dde2660b8d50bfa4fa0b03b9)

## 方法

方法可以被前向引用，但这不是一种特殊的技术
这是因为动态检查方法的存在
(在 Erg 中，方法存在是静态检查的。对于前向引用，函数必须是常量。)

```python
>>> class C:
...   def f(self, x):
...       if x == 0: return 0
...       else: return self.g(x)
...   def g(self, x): return self.f(x - 1)
```

## 继承，覆盖

一些被覆盖的方法 m 被简单地覆盖，就像变量重新分配一样
在父类中引用 m 的方法将在子类中引用被覆盖的 m

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

因此，即使它显然被错误地覆盖，直到运行时才会出错

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
类型错误: 只能将str(不是"int")连接到str
```

Erg 静态检查与父类的一致性
重写时必须给出"Override"装饰器，并且重写函数的类型必须是被重写函数类型的子类型

```python
>>> C = Class()
...   .f self = 1
...   .g self = self.f() + 1
...
>>> D = Inherit C
...   .f self = "a"
...
错误[#XX]: 文件"<stdin>"，第 5 行，在 D 中
要覆盖 f，必须添加 `Override` 装饰器，其类型必须是 `Self.() -> Nat` 或其子类型
f(self) 已在 C 中定义。要覆盖 f，必须添加 `Override` 装饰器，其类型必须为 `Self. 要覆盖，必须给它一个 `Override` 装饰器，并且它的类型必须是 `Self.() -> Nat` 或 that.f(self) 的子类型
```

## 类型检查

类型检查通常都是关于检查函数参数的类型
在 Python 中，大多数操作都是方法调用。如果对象所属的类在调用时没有附加方法，就是这样

```python
def f(x):
    return x.m()

class C:
    def m(self): return None

c = C()
f(c)
f(1) # 类型错误
```

```python
# f: |T, X <: {.m = Self.() -> T}| X -> T
f(x) = x.m()

C = Class()
C.m(self) = None

c = C.new()
f(c)
f(1) # 类型错误: f 将具有方法 `.m` 的类型作为参数，但传递了 Nat
```
