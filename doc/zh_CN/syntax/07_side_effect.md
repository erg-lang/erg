# 副作用和程序

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/07_side_effect.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/07_side_effect.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

我们一直忽略了解释"！"的含义，但现在它的含义终于要揭晓了。这个 `!` 表示这个对象是一个带有"副作用"的"过程"。过程是具有副作用的函数

```python,compile_fail
f x = print! x # EffectError: 不能为函数分配有副作用的对象
# 提示: 将名称更改为 'f!'
```

上面的代码会导致编译错误。这是因为您在函数中使用了过程。在这种情况下，您必须将其定义为过程

```python
p! x = print! x
```

`p!`, `q!`, ... 是过程的典型变量名
以这种方式定义的过程也不能在函数中使用，因此副作用是完全隔离的

## 方法

函数和过程中的每一个都可以是方法。函数式方法只能对`self`进行不可变引用，而程序性方法可以对`self`进行可变引用
`self` 是一个特殊的参数，在方法的上下文中是指调用对象本身。引用 `self` 不能分配给任何其他变量

```python,compile_fail
C!.
    method ref self =
        x = self # 所有权错误: 无法移出`self`
        x
```

程序方法也可以采取 `self` 的 [ownership](./18_ownership.md)。从方法定义中删除 `ref` 或 `ref!`

```python,compile_fail
n = 1
s = n.into(Str) # '1'
n # 值错误: n 被 .into 移动(第 2 行)
```

在任何给定时间，只有一种程序方法可以具有可变引用。此外，在获取可变引用时，不能从原始对象获取更多可变引用。从这个意义上说，`ref!` 会对`self` 产生副作用

但是请注意，可以从可变引用创建(不可变/可变)引用。这允许在程序方法中递归和 `print!` 的`self`

```python
T -> T # OK (move)
T -> Ref T # OK (move)
T => Ref! T # OK (only once)
Ref T -> T # NG
Ref T -> Ref T # OK
Ref T => Ref!
T -> Ref T # NG
T -> Ref T # OK
T => Ref!
```

## 附录: 副作用的严格定义

代码是否具有副作用的规则无法立即理解
直到你能理解它们，我们建议你暂时把它们定义为函数，如果出现错误，添加`！`将它们视为过程
但是，对于那些想了解该语言的确切规范的人，以下是对副作用的更详细说明

首先，必须声明返回值的等价与 Erg 中的副作用无关
有些过程对于任何给定的 `x` 都会导致 `p!(x) == p!(x)`(例如，总是返回 `None`)，并且有些函数会导致 `f(x) ！ = f(x)`

前者的一个例子是`print!`，后者的一个例子是下面的函数

```python
nan _ = Float.NaN
assert nan(1) ! = nan(1)
```

还有一些对象，例如类，等价确定本身是不可能的

```python
T = Structural {i = Int}
U = Structural {i = Int}
assert T == U

C = Class {i = Int}
D = Class {i = Int}
assert C == D # 类型错误: 无法比较类
```

言归正传: Erg 中"副作用"的准确定义是

* 访问可变的外部信息

"外部"一般是指外部范围； Erg 无法触及的计算机资源和执行前/执行后的信息不包含在"外部"中。"访问"包括阅读和写作

例如，考虑 `print!` 过程。乍一看，`print!` 似乎没有重写任何变量。但如果它是一个函数，它可以重写外部变量，例如，使用如下代码: 

```python
camera = import "some_camera_module"
ocr = import "some_ocr_module"

n = 0
_ =
    f x = print x # 假设我们可以使用 print 作为函数
    f(3.141592)
cam = camera.new() # 摄像头面向 PC 显示器
image = cam.shot!()
n = ocr.read_num(image) # n = 3.141592
```

将"camera"模块视为为特定相机产品提供 API 的外部库，将"ocr"视为用于 OCR(光学字符识别)的库
直接的副作用是由 `cam.shot!()` 引起的，但显然这些信息是从 `f` 泄露的。因此，`print!` 本质上不可能是一个函数

然而，在某些情况下，您可能希望临时检查函数中的值，而不想为此目的在相关函数中添加 `!`。在这种情况下，可以使用 `log` 函数
`log` 打印整个代码执行后的值。这样，副作用就不会传播

```python
log "this will be printed after execution"
print! "this will be printed immediately"
# 这将立即打印
# 这将在执行后打印
```

如果没有反馈给程序，或者换句话说，如果没有外部对象可以使用内部信息，那么信息的"泄漏"是可以允许的。只需要不"传播"信息

<p align='center'>
    <a href='./06_operator.md'>上一页</a> | <a href='./08_procedure.md'>下一页</a>
</p>
