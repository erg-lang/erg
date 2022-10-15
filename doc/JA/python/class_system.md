# Pythonのクラスシステム(Ergとの比較)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/class_system.md%26commit_hash%3D2ecd249a2a99dc93dde2660b8d50bfa4fa0b03b9)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/class_system.md&commit_hash=2ecd249a2a99dc93dde2660b8d50bfa4fa0b03b9)

## メソッド

メソッドは前方参照していてもかまわないが、これは特別なテクニックが使われているわけではなく、
メソッドの実在が動的に検査されるためである。
(Ergではメソッドの実在は静的に検査される。前方参照するためには関数を定数にしなくてはならない。)

```python
>>> class C:
...   def f(self, x):
...       if x == 0: return 0
...       else: return self.g(x)
...   def g(self, x): return self.f(x - 1)
```

## 継承、オーバーライド

オーバーライドされたあるメソッドmは単に変数の再代入のように上書きされ、
親クラスのmを参照するメソッドは子クラスではオーバーライドされたmを参照するようになる。

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

なので、明らかに間違ってオーバーライドされても実行時までエラーとならない。

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

Ergでは親クラスとの整合性が静的に検査される。
オーバーライド時には`Override`デコレータを付与する必要があり、オーバーライドする関数の型はされる関数の型の部分型とならなくてはならない。

```python
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

## 型チェック

型チェックは概ね関数引数の型チェックに尽きる。
Pythonでは、大半の操作がメソッド呼び出しである。呼び出し時にオブジェクトの属するクラスにメソッドがついていなればそれまでである。

```python
def f(x):
    return x.m()

class C:
    def m(self): return None

c = C()
f(c)
f(1) # TypeError
```

```python
# f: |T, X <: {.m = Self.() -> T}| X -> T
f(x) = x.m()

C = Class()
C.m(self) = None

c = C.new()
f(c)
f(1) # TypeError: f takes a type has method `.m` as an argument, but passed Nat
```
