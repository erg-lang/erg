# Python 字節碼指令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/bytecode_instructions.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/bytecode_instructions.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

Python 字節碼變量操作命令通過 名稱索引(名稱索引)訪問。 這是為了在 Python 中實現動態變量訪問(可以使用 eval 等作為字符串訪問)。
一條指令為 2 個字節，指令和參數以 little endian 形式存儲。
不帶參數的指令也使用 2 個字節(參數部分為 0)。

## STORE_NAME(名稱索引)

```python
globals[namei] = stack.pop()
```

## LOAD_NAME(名稱索引)

```python
stack.push(globals[namei])
```

Only called at top level.

## LOAD_GLOBAL(名稱索引)

```python
stack.push(globals[namei])
```

用于加載內部作用域頂層的STORE_NAME，但頂層的`名稱索引`不一定與某個作用域的代碼對象中的名稱索引相同(名稱相同，名稱索引不一定)

## LOAD_CONST(名稱索引)

```python
stack.push(consts[namei])
```

在常量表中加載常量。
目前(Python 3.9)，在 CPython 中，每個 lambda 函數都是 MAKE_FUNCTION，名稱為"\<lambda\>"

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

## STORE_FAST(名稱索引)

fastlocals[namei] = stack.pop()
可能對應于頂層的 STORE_NAME
假定未引用(或單個)變量由此存儲
全局空間有自己的指令是為了優化嗎?

## LOAD_FAST(名稱索引)

```python
stack.push(fastlocals[namei])
```
fastlocals 是變量名嗎?

## LOAD_CLOSURE(名稱索引)

```python
cell = freevars[namei]
stack. push(cell)
```

然后調用 BUILD_TUPLE
它只在閉包內被調用，并且 cellvars 應該在閉包內存儲引用。
與 LOAD_DEREF 不同，每個單元格(填充有引用的容器)都被推入堆棧

## STORE_DEREF(名稱索引)

```python
cell = freevars[namei]
cell.set(stack.pop())
```

在內部范圍內沒有引用的變量是 STORE_FAST，但引用的變量是 STORE_DEREF
在 Python 中，引用計數在該指令內遞增和遞減

## LOAD_DEREF(名稱索引)

```python
cell = freevars[namei]
stack.push(cell.get())
```

## 名稱列表

### 變量名

fast_locals 對應的函數內部變量名稱列表
即使名稱中有同名的變量，它們也基本不一樣(新創建的和外部變量不能從那個范圍訪問)
即沒有在范圍內定義的外部引用的變量進入 varnames

### 名字

與全局兼容
范圍內使用的外部常量(僅引用)的名稱列表(在頂層，即使是普通變量也包含在名稱中)
即在范圍之外定義的常量進入名稱

## 自由變量

與免費變量兼容
閉包捕獲的變量。它在同一個函數實例中靜態地運行。

## 單元格變量

對應于 cellvars
在函數內捕獲到內部閉包函數的變量。 由于制作了副本，因此原始變量保持原樣。