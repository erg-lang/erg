# Python Bytecode Instructions

Python bytecode 的变量操作系统的指令通过 namei（name index）进行访问。这是为了实现 Python 的动态变量访问（可以使用 eval 等以字符串访问）。1 命令为 2byte，命令、自变量用 little endian 收纳。不取参数的命令也使用 2byte（参数部分为 0）。

## STORE_NAME(namei)


```python
globals[namei] = stack.pop()
```

## LOAD_NAME(namei)


```python
stack.push(globals[namei])
```

只能在顶层调用。

## LOAD_GLOBAL(namei)


```python
stack.push(globals[namei])
```

这是为了在内部作用域中 Load 在顶层 STORE_NAME 后的内容，但如果在顶层，则与某个作用域代码对象中的 namei 不一定相同（名称相同，而不是 namei）

## LOAD_CONST(namei)


```python
stack.push(consts[namei])
```

从常量表中加载常量。目前（Python 3.9），CPython 将每个 Lambda 函数都命名为“\”


```console
>>> dis.dis("[1,2,3].map(lambda x: x+1)")
1       0 LOAD_CONST               0 (1)
        2 LOAD_CONST               1 (2)
        4 LOAD_CONST               2 (3)
        6 BUILD_LIST               3
        8 LOAD_ATTR                0 (map)
        10 LOAD_CONST               3 (<code object <lambda> at 0x7f272897fc90, file "<dis>", line 1>)
        12 LOAD_CONST               4 ('<lambda>')
        14 MAKE_FUNCTION            0
        16 CALL_FUNCTION            1
        18 RETURN_VALUE
```

## STORE_FAST(namei)

fastlocals[namei]=stack.pop（）可能没有（或单个）与顶级 STORE_NAME 相对应的参照的变量被认为是这样存储的特意全局空间有自己的指令是为了优化？

## LOAD_FAST(namei)

stack.push（fastlocals[namei] ）fastlocals 是 varnames？

## LOAD_CLOSURE(namei)


```python
cell = freevars[namei]
stack.push(cell)
```

然后，只有在调用 BUILD_TUPLE 的闭包中才会调用 BUILD_TUPLE，cellvars 将每个 cell（包含引用的容器）push 到堆栈中，而 LOAD_DEREF 似乎存储闭包中的引用

## STORE_DEREF(namei)


```python
cell = freevars[namei]
cell.set(stack.pop())
```

内部作用域中没有参照的变量被 STORE_FAST，但是被参照的变量被 STORE_DEREF 的 Python 中，在这个命令内进行参照计数的增减

## LOAD_DEREF(namei)


```python
cell = freevars[namei]
stack.push(cell.get())
```

## 名称列表

### varnames

与 fast_locals 相对应的函数内部变量的名称列表 names 中具有相同名称的变量基本上不相同（新创建的变量，不能从该范围访问外部变量），即没有在范围内定义的外部参照的变量将包含在 varnames 中

### names

与 globals 相对应范围内使用的外部常量（仅引用）的名称列表（即使在顶层是普通变量，也会在 names 中）即，范围外定义的常量会在 names 中

## free variable

对应于 freevars 的闭包捕获的变量。在同一函数实例内进行 static 行为。

## cell variables

cellvars 对应函数内部闭包函数捕获的变量。复制后，原始变量保持不变。
