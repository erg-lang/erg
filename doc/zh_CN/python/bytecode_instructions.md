# Python 字节码指令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/bytecode_instructions.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/bytecode_instructions.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

Python 字节码变量操作命令通过 名称索引(名称索引)访问。 这是为了在 Python 中实现动态变量访问(可以使用 eval 等作为字符串访问)。
一条指令为 2 个字节，指令和参数以 little endian 形式存储。
不带参数的指令也使用 2 个字节(参数部分为 0)。

## STORE_NAME(名称索引)

```python
globals[namei] = stack.pop()
```

## LOAD_NAME(名称索引)

```python
stack.push(globals[namei])
```

Only called at top level.

## LOAD_GLOBAL(名称索引)

```python
stack.push(globals[namei])
```

用于加载内部作用域顶层的STORE_NAME，但顶层的`名称索引`不一定与某个作用域的代码对象中的名称索引相同(名称相同，名称索引不一定)

## LOAD_CONST(名称索引)

```python
stack.push(consts[namei])
```

在常量表中加载常量。
目前(Python 3.9)，在 CPython 中，每个 lambda 函数都是 MAKE_FUNCTION，名称为“\<lambda\>”

```console
>>> dis.dis("[1,2,3].map(lambda x: x+1)")
1 0 LOAD_CONST 0 (1)
        2 LOAD_CONST 1 (2)
        4 LOAD_CONST 2 (3)
        6 BUILD_LIST 3
        8 LOAD_ATTR 0 (map)
        10 LOAD_CONST 3 (<code object <lambda> at 0x7f272897fc90, file "<dis>", line 1>)
        12 LOAD_CONST 4 ('<lambda>')
        14 MAKE_FUNCTION 0
        16 CALL_FUNCTION 1
        18 RETURN_VALUE
```

## STORE_FAST(名称索引)

fastlocals[namei] = stack.pop()
可能对应于顶层的 STORE_NAME
假定未引用(或单个)变量由此存储
全局空间有自己的指令是为了优化吗?

## LOAD_FAST(名称索引)

```python
stack.push(fastlocals[namei])
```
fastlocals 是变量名吗?

## LOAD_CLOSURE(名称索引)

```python
cell = freevars[namei]
stack. push(cell)
```

然后调用 BUILD_TUPLE
它只在闭包内被调用，并且 cellvars 应该在闭包内存储引用。
与 LOAD_DEREF 不同，每个单元格(填充有引用的容器)都被推入堆栈

## STORE_DEREF(名称索引)

```python
cell = freevars[namei]
cell.set(stack.pop())
```

在内部范围内没有引用的变量是 STORE_FAST，但引用的变量是 STORE_DEREF
在 Python 中，引用计数在该指令内递增和递减

## LOAD_DEREF(名称索引)

```python
cell = freevars[namei]
stack.push(cell.get())
```

## 名称列表

### 变量名

fast_locals 对应的函数内部变量名称列表
即使名称中有同名的变量，它们也基本不一样(新创建的和外部变量不能从那个范围访问)
即没有在范围内定义的外部引用的变量进入 varnames

### 名字

与全局兼容
范围内使用的外部常量(仅引用)的名称列表(在顶层，即使是普通变量也包含在名称中)
即在范围之外定义的常量进入名称

## 自由变量

与免费变量兼容
闭包捕获的变量。它在同一个函数实例中静态地运行。

## 单元格变量

对应于 cellvars
在函数内捕获到内部闭包函数的变量。 由于制作了副本，因此原始变量保持原样。