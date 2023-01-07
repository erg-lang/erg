# 与 Python 集成

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/33_integration_with_Python.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/33_integration_with_Python.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

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

默认情况下，从 Python 导入的所有对象都是"Object"类型。由于此时无法进行比较，因此有必要细化类型

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
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```

```python
foo = pyimport "foo"
assert foo.bar(1) in Int
```

这通过在运行时执行类型检查来确保类型安全。``declare`` 函数大致如下工作

```python
declare|S: Subroutine| sub!: S, T =
    # 实际上，=> 可以强制转换为没有块副作用的函数
    x =>
        assert x in T.Input
        y = sub!(x)
        assert y in T.Output
        y
```

由于这是运行时开销，因此计划使用 Erg 的类型系统对 Python 脚本进行静态类型分析

<p align='center'>
    <a href='./32_pipeline.md'>上一页</a> | <a href='./34_package_system.md'>下一页</a>
</p>
