# 功能

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/funcs.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/funcs.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## 基本功能

### if|T; U|(cond: Bool, then: T, else: U) -> T or U

### map|T; U|(i: Iterable T, f: T -> U) -> Map U

请注意，参数的顺序与 Python 相反

### log(x: Object, type: LogType = Info) -> None

在调试显示中记录"x"。执行完成后汇总并显示日志
支持表情符号的终端根据"类型"添加前缀

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

### panic(msg: Str) -> Panic

显示msg并停止
支持表情符号的终端有一个🚨前缀

### discard|T|(x: ...T) -> NoneType

扔掉`x`。不使用返回值时使用。与 `del` 不同，它不会使变量 `x` 不可访问

```python
p! x =
    # q!应该返回一些不是None或()的值
    # 如果不需要，请使用`discard`
    discard q!(x)
    f x

discard True
assert True # OK
```

### import(path: Path) -> Module or CompilerPanic

导入一个模块。如果找不到模块，则引发编译错误

### eval(code: Str) -> Object

将`code`作为代码进行评估并返回

### classof(object: Object) -> Class

返回`object`的类
但是，由于无法比较类，如果要判断实例，请使用`object in Class`而不是`classof(object) == Class`
编译时确定的结构类型是通过`Typeof`获得的

## Iterator, Array生成系统

### repeat|T|(x: T) -> RepeatIterator T

```python
rep = repeat 1 # Repeater(1)
for! rep, i =>
    print! i
# 1 1 1 1 1 ...
```

### dup|T; N|(x: T, N: Nat) -> [T; N]

```python
[a, b, c] = dup new(), 3
print! a # <Object object>
print! a == b # False
```

### cycle|T|(it: Iterable T) -> CycleIterator T

```python
cycle([0, 1]).take 4 # [0, 1, 0, 1]
cycle("hello").take 3 # "hellohellohello"
```

## 定数式関数

### Class

创建一个新类。与`Inherit`不同，通过`Class`传递与基类型无关，并且方法会丢失
您将无法进行比较，但您可以进行模式匹配等操作

```python
C = Class {i = Int}
NewInt = Class Int
Months = Class 1..12
jan = Months.new(1)
jan + Months.new(2) # TypeError: `+` is not implemented for 'Months'
match jan:
    1 -> log "January"
    _ -> log "Other"
```

第二个参数 Impl 是要实现的Trait

### Inherit

继承一个类。您可以按原样使用基类方法

### Trait

创造一个新的trait。目前，只能指定记录类型

### Typeof

返回参数类型。如果要获取运行时类，请使用`classof`
如果您将其用于类型规范，则会出现警告

```python
x: Typeof i = ...
# TypeWarning: Typeof(i) == Int, please replace it
```

### Deprecated

作为解码器使用。警告不推荐使用的类型或函数