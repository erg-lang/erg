# 與 Python 集成

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_integration_with_Python.md%26commit_hash%3Da2bad2c8f14b1e33c22229e687b71ce02858739a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_integration_with_Python.md&commit_hash=a2bad2c8f14b1e33c22229e687b71ce02858739a)

## 導出到 Python

編譯 Erg 腳本時，會生成一個 .pyc 文件，可以簡單地將其作為 Python 模塊導入
但是，無法從 Python 訪問在 Erg 端設置為私有的變量

```python
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```

```console
erg --compile foo.er
```

```python,checker_ignore
import foo

print(foo.public)
print(foo.private) # 屬性錯誤:
```

## 從 Python 導入

默認情況下，從 Python 導入的所有對象都是"Object"類型。由于此時無法進行比較，因此有必要細化類型

## 標準庫中的類型規范

Python 標準庫中的所有 API 都是由 Erg 開發團隊指定的類型

```python
time = pyimport "time"
time.sleep! 1
```

## 用戶腳本的類型規范

創建一個類型為 Python `foo` 模塊的 `foo.d.er` 文件
Python 端的類型提示被忽略，因為它們不是 100% 保證的

```python
# foo.py
X = ...
def bar(x):
    ...
def baz():
    ...
...
```

```python
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```

```python
foo = pyimport "foo"
assert foo.bar(1) in Int
```

這通過在運行時執行類型檢查來確保類型安全。``declare`` 函數大致如下工作

```python
declare|S: Subroutine| sub!: S, T =
    # 實際上，=> 可以強制轉換為沒有塊副作用的函數
    x =>
        assert x in T.Input
        y = sub!(x)
        assert y in T.Output
        y
```

由于這是運行時開銷，因此計劃使用 Erg 的類型系統對 Python 腳本進行靜態類型分析

## Overloading

A special type that can be used only with Python typing is the overloaded type. This is a type that can accept multiple types.

```python
f: (Int -> Str) and (Str -> Int)
```

Overloaded types can be declared by taking a subroutine type intersection (`and`, not union `or`).

This allows you to declare a function whose return type depends on the type of its arguments.

```python
f(1): Str
f("1"): Int
```

The type decisions are collated from left to right, and the first match is applied.

Such polymorphism is called ad hoc polymorphism and is different from Erg's polymorphism, which uses type variables and trait bounds. Ad hoc polymorphism is generally discouraged, but it is a necessary  because of its universal use in Python code.

Parameter types of overloaded types may be in a subtype relationship and may have different number of parameters, but they must not be of the same type, i.e. return type overload is not allowed.

```python
# OK
f: (Nat -> Str) and (Int -> Int)
f: ((Int, Int) -> Str) and (Int -> Int)
```

```python,compile_fail
# NG
f: (Int -> Str) and (Int -> Int)
```

## Declaration of Trait Implementation

To implement a trait and declare trait members for a class, write the following (taken from [type declarations for numpy.NDArray](https://github.com/erg-lang/erg/blob/main/crates/erg_compiler/lib/external/numpy.d/__init__.d.er)).

```erg
.NDArray = 'ndarray': (T: Type, Shape: [Nat; _]) -> ClassType
...
.NDArray(T, S)|<: Add .NDArray(T, S)|.
    Output: {.NDArray(T, S)}
    __add__: (self: .NDArray(T, S), other: .NDArray(T, S)) -> .NDArray(T, S)
```

## Notes

Currently, Erg unconditionally trusts the contents of type declarations. In other words, you can declare a variable of type `Str` even if it is actually a variable of type `Int`, or declare a subroutine as a function even if it has side effects, etc.

Also, it is troublesome that type declarations cannot be omitted even for trivial code, so the [Project for static type analysis of Python scripts with Erg's type system](https://github.com/mtshiba/pylyzer) is underway.

<p align='center'>
    <a href='./33_pipeline.md'>上一頁</a> | <a href='./35_package_system.md'>下一頁</a>
</p>
