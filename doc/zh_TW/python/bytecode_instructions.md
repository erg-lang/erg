# Python Bytecode Instructions

Python bytecode 的變量操作系統的指令通過 namei（name index）進行訪問。這是為了實現 Python 的動態變量訪問（可以使用 eval 等以字符串訪問）。 1 命令為 2byte，命令、自變量用 little endian 收納。不取參數的命令也使用 2byte（參數部分為 0）。

## STORE_NAME(namei)


```python
globals[namei] = stack.pop()
```

## LOAD_NAME(namei)


```python
stack.push(globals[namei])
```

只能在頂層調用。

## LOAD_GLOBAL(namei)


```python
stack.push(globals[namei])
```

這是為了在內部作用域中 Load 在頂層 STORE_NAME 後的內容，但如果在頂層，則與某個作用域代碼對像中的 namei 不一定相同（名稱相同，而不是 namei）

## LOAD_CONST(namei)


```python
stack.push(consts[namei])
```

從常量表中加載常量。目前（Python 3.9），CPython 將每個 Lambda 函數都命名為“\”


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

fastlocals[namei]=stack.pop（）可能沒有（或單個）與頂級 STORE_NAME 相對應的參照的變量被認為是這樣存儲的特意全局空間有自己的指令是為了優化？

## LOAD_FAST(namei)

stack.push（fastlocals[namei] ）fastlocals 是 varnames？

## LOAD_CLOSURE(namei)


```python
cell = freevars[namei]
stack.push(cell)
```

然後，只有在調用 BUILD_TUPLE 的閉包中才會調用 BUILD_TUPLE，cellvars 將每個 cell（包含引用的容器）push 到堆棧中，而 LOAD_DEREF 似乎存儲閉包中的引用

## STORE_DEREF(namei)


```python
cell = freevars[namei]
cell.set(stack.pop())
```

內部作用域中沒有參照的變量被 STORE_FAST，但是被參照的變量被 STORE_DEREF 的 Python 中，在這個命令內進行參照計數的增減

## LOAD_DEREF(namei)


```python
cell = freevars[namei]
stack.push(cell.get())
```

## 名稱列表

### varnames

與 fast_locals 相對應的函數內部變量的名稱列表 names 中具有相同名稱的變量基本上不相同（新創建的變量，不能從該範圍訪問外部變量），即沒有在範圍內定義的外部參照的變量將包含在 varnames 中

### names

與 globals 相對應範圍內使用的外部常量（僅引用）的名稱列表（即使在頂層是普通變量，也會在 names 中）即，範圍外定義的常量會在 names 中

## free variable

對應於 freevars 的閉包捕獲的變量。在同一函數實例內進行 static 行為。

## cell variables

cellvars 對應函數內部閉包函數捕獲的變量。複製後，原始變量保持不變。