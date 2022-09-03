# 与 Python 合作

## 导出到 Python

编译 Erg 脚本将生成一个.pyc 文件，你可以将其作为一个模块导入 Python。但是，在 Erg 端设置为私有的变量不能从 Python 访问。


```erg
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```


```console
erg --compile foo.er
```


```python
import foo

print(foo.public)
print(foo.private) # AttributeError:
```

## 从 Python 导入

默认情况下，从 Python 引入的所有对象都是类型。长此以往，我们也无法进行比较，所以我们需要进行类型的筛选。

## 标准库类型

Python 标准库中的所有 API 都由 Erg 开发团队指定类型。


```erg
time = pyimport "time"
time.sleep! 1
```

## 指定用户脚本类型

创建一个文件，为 Python 的<gtr=“10”/>模块创建类型。Python 端的 type hint 不是 100% 的保证，因此将被忽略。


```python
# foo.py
X = ...
def bar(x):
    ...
def baz():
    ...
```


```erg
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```


```erg
foo = pyimport "foo"
assert foo.bar(1) in Int
```

它通过在运行时执行类型检查来保证类型安全性。函数的工作原理大致如下。


```erg
declare|S: Subroutine| sub!: S, T =
    # 実は、=>はブロックの副作用がなければ関数にキャストできる
    x =>
        assert x in T.Input
        y = sub!(x)
        assert y in T.Output
        y
```

这是一个运行时开销，因此计划在 Erg 类型系统上对 Python 脚本进行静态类型分析。

<p align='center'>
    <a href='./31_pipeline.md'>Previous</a> | <a href='./33_package_system.md'>Next</a>
</p>
