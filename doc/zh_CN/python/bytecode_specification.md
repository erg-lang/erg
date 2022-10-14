# Python 字节码规范

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/bytecode_specification.md%26commit_hash%3D9f6a4a43fcf7e4f58cabe6e5a7546820fd9f5ff4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/bytecode_specification.md&commit_hash=9f6a4a43fcf7e4f58cabe6e5a7546820fd9f5ff4)


## 格式

* 0~3   字节(u32): 幻数(详见common/bytecode.rs)
* 4~7   字节(u32): 0 padding
* 8~12  字节(u32): 时间戳
* 13~   字节(PyCodeObject): 代码对象

## PyCode 对象

* 0     字节(u8): '0xe3' (前缀，这意味着代码的'c')
* 01~04 字节(u32): args个数(co_argcount)
* 05~08 字节(u32): position-only args 的数量 (co_posonlyargcount)
* 09~12 字节(u32): 仅关键字参数的数量(co_kwonlyargcount)
* 13~16 字节(u32): 本地数 (co_nlocals)
* 17~20 字节(u32): 栈大小(co_stacksize)
* 21~24 字节(u32): 标志(co_flags)()
* ?     字节: 字节码指令，以'0x53'、'0x0'结尾(83, 0): RETURN_VALUE(co_code)
* ?     字节(PyTuple): 代码中使用的常量(co_consts)
* ?     字节(PyTuple): 代码中使用的名称(co_names)
* ?     字节(PyTuple): 代码中定义的变量名，包括params (PyTuple) (co_varnames)
* ?     字节(PyTuple): 从外部范围捕获的变量(co_freevars)
* ?     字节(PyTuple): 内部闭包中使用的变量(co_cellvars)
* ?     字节(PyUnicode 或 PyShortAscii): 文件名，它是从哪里加载的(co_filename)
* ?     字节(PyUnicode or PyShortAscii): 代码本身的名字，默认是\<module\> (co_name)
* ?~?+3 字节(u32): 第一行数 (co_firstlineno)
* ?     字节(bytes): 行表，用 PyStringObject? (co_lnotab)

## Py 元组对象

* 0     字节: 0x29 (意思是:`)`)
* 01~04 字节(u32): 元组项数
* ?     字节(PyObject): 项目

## PyString 对象

* 如果我使用 ascii 以外的字符，它会变成 PyUnicode 吗?
* "あ"、"𠮷"和"α"是 PyUnicode(不再使用?)

* 0     字节: 0x73(表示`s`)
* 1~4   字节: 字符串长度
* 5~    字节: 有效载荷

## PyUnicode 对象

* 0     字节: 0x75(表示`u`)
* 1~4   字节: 字符串长度
* 5~    字节: 有效载荷

## PyShortAscii 对象

* 这叫短，但是即使超过100个字符，仍然会保持在短的状态
* 或者更确切地说，没有不短的 ascii(短数据类型吗?)

* 0     字节: 0xFA(表示`z`)
* 1~4   字节: 字符串长度
* 5~    字节: 有效载荷

## PyInterned 对象

* 实习对象注册在专用地图中，可以与is进行比较
* 例如字符串，无论其长度如何，都可以在恒定时间内进行比较

* 0     字节: 0x74(表示`t`)

## PyShortAsciiInterned 对象

* 0     字节: 0xDA(表示`Z`)
* 1~4   字节: 字符串长度
* 5~    字节: 有效载荷