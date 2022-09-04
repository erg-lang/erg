# Python class system (compare with Erg)

## Methods

Methods may be forward referenced, but this is not a special technique used.
This is because the existence of a method is dynamically checked.
(In Erg, method existence is checked statically. For forward references, functions must be constants.)

```python
>>> class C:
...   def f(self, x):
...       if x == 0: return 0
...       else: return self.g(x)
...   def g(self, x): return self.f(x - 1)
```

## Inheritance, overriding

Some overridden method m is simply overwritten, like a variable reassignment.
A method that refers to m in the parent class will refer to the overridden m in the child class.

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

So, even if it is overridden by mistake obviously, it will not be an error until runtime.

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
TypeError: can only concatenate str (not "int") to str
```

Erg statically checks for consistency with the parent class.
The `Override` decorator must be given when overriding, and the type of the overriding function must be a subtype of the type of the function being overridden.

```erg
>>> C = Class()
...   .f self = 1
...   .g self = self.f() + 1
...
>>> D = Inherit C
...   .f self = "a"
...
Error[#XX]: File "<stdin>", line 5, in D
To override f, it must be added `Override` decorator and its type must be `Self.() -> Nat` or the subtype of that
f(self) is already defined in C. To override f, it must be added `Override` decorator and its type must be `Self. To override, it must be given an `Override` decorator and its type must be `Self.() -> Nat` or the subtype of that.f(self).
```

## Type checking

Type checking is generally all about checking the type of function arguments.
In Python, most operations are method calls. If the class to which the object belongs does not have a method attached to it at the time of the call, that's it.

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
