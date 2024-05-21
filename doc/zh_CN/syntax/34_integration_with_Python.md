# 与 Python 集成

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_integration_with_Python.md%26commit_hash%3Da2bad2c8f14b1e33c22229e687b71ce02858739a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_integration_with_Python.md&commit_hash=a2bad2c8f14b1e33c22229e687b71ce02858739a)

## 导出到 Python

编译 Erg 脚本时，会生成一个 .pyc 文件，可以简单地将其作为 Python 模块导入
但是，无法从 Python 访问在 Erg 端设置为私有的变量

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
print(foo.private) # 属性错误:
```

## 从 Python 导入

默认情况下，从 Python 导入的所有对象都是`Object`类型。由于此时无法进行比较，因此有必要细化类型

## 标准库中的类型规范

Python 标准库中的所有 API 都是由 Erg 开发团队指定的类型

```python
time = pyimport "time"
time.sleep! 1
```

## 用户脚本的类型规范

创建一个类型为 Python `foo` 模块的 `foo.d.er` 文件
Python 端的类型提示被忽略，因为它们不是 100% 保证的

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
.X: Int
.bar!: Int => Int
.foo! = baz!: () => Int # aliasing
.C!: Class
```

No syntax other than declarations and definitions (aliasing) are allowed in d.er.

If an identifier on the Python side is not a valid identifier in Erg, it can be escaped by enclosing it in single quotes (').

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

## Notes

Currently, Erg unconditionally trusts the contents of type declarations. In other words, you can declare a variable of type `Str` even if it is actually a variable of type `Int`, or declare a subroutine as a function even if it has side effects, etc.

Also, it is troublesome that type declarations cannot be omitted even for trivial code, so the [Project for static type analysis of Python scripts with Erg's type system](https://github.com/mtshiba/pylyzer) is underway.

<p align='center'>
    <a href='./34_pipeline.md'>上一页</a> | <a href='./35_package_system.md'>下一页</a>
</p>
