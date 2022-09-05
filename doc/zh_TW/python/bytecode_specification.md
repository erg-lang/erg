# Python bytecode specification

## Format

* 0~3 byte(u32): magic number (see common/bytecode.rs for details)
* 4~7 byte(u32): 0 padding
* 8~12 byte(u32): timestamp
* 13~ byte(PyCodeObject): code object

## PyCodeObject

* 0     byte(u8): '0xe3' (prefix, this means code's 'c')
* 01~04 byte(u32): number of args (co_argcount)
* 05~08 byte(u32): number of position-only args (co_posonlyargcount)
* 09~12 byte(u32): number of keyword-only args (co_kwonlyargcount)
* 13~16 byte(u32): number of locals (co_nlocals)
* 17~20 byte(u32): stack size (co_stacksize)
* 21~24 byte(u32): flags (co_flags) ()
* ?     byte: bytecode instructions, ends with '0x53', '0x0' (83, 0): RETURN_VALUE (co_code)
* ?     byte(PyTuple): constants used in the code (co_consts)
* ?     byte(PyTuple): names used in the code (co_names)
* ?     byte(PyTuple): variable names defined in the code, include params (PyTuple) (co_varnames)
* ?     byte(PyTuple): variables captured from the outer scope (co_freevars)
* ?     byte(PyTuple): variables used in the inner closure (co_cellvars)
* ?     byte(PyUnicode or PyShortAscii): file name, where it was loaded from (co_filename)
* ?     byte(PyUnicode or PyShortAscii): the name of code itself, default is \<module\> (co_name)
* ?~?+3 byte(u32): number of first line (co_firstlineno)
* ?     byte(bytes): line table, represented by PyStringObject? (co_lnotab)

## PyTupleObject

* 0     byte: 0x29 (means ')')
* 01~04 byte(u32): number of tuple items
* ?     byte(PyObject): items

## PyStringObject

* If I use a character other than ascii, does it become PyUnicode?
* "あ", "??", and "α" are PyUnicode (no longer used?)

* 0     byte: 0x73 (means 's')
* 1~4   byte: length of string
* 5~    byte: payload

## PyUnicodeObject

* 0     byte: 0x75 (means 'u')
* 1~4   byte: length of string
* 5~    byte: payload

## PyShortAsciiObject

* This is called short, but even if there are more than 100 characters, this will still short
* or rather, there is no ascii that is not short (is short a data type?)

* 0     byte: 0xFA (means 'z')
* 1~4   byte: length of string
* 5~    byte: payload

## PyInternedObject

* interned objects are registered in a dedicated map and can be compared with is
* String, for example, can be compared in constant time regardless of its length

* 0     byte: 0x74 (means 't')

## PyShortAsciiInternedObject

* 0     byte: 0xDA (means 'Z')
* 1~4   byte: length of string
* 5~    byte: payload


# Python 字節碼規范

## 格式

* 0~3 byte(u32)：幻數(詳見common/bytecode.rs)
* 4~7 byte(u32): 0 padding
* 8~12 byte(u32): 時間戳
* 13~ byte(PyCodeObject): 代碼對象

## PyCode 對象

* 0     byte(u8): '0xe3' (前綴，這意味著代碼的'c')
* 01~04 byte(u32): args個數(co_argcount)
* 05~08 byte(u32): position-only args 的數量 (co_posonlyargcount)
* 09~12 byte(u32)：僅關鍵字參數的數量(co_kwonlyargcount)
* 13~16 byte(u32): 本地數 (co_nlocals)
* 17~20 byte(u32): 棧大小(co_stacksize)
* 21~24 byte(u32)：標志(co_flags)()
* ?     byte：字節碼指令，以'0x53'、'0x0'結尾(83, 0)：RETURN_VALUE(co_code)
* ?     byte(PyTuple)：代碼中使用的常量(co_consts)
* ?     byte(PyTuple)：代碼中使用的名稱(co_names)
* ?     byte(PyTuple)：代碼中定義的變量名，包括params (PyTuple) (co_varnames)
* ?     byte(PyTuple)：從外部范圍捕獲的變量(co_freevars)
* ?     byte(PyTuple)：內部閉包中使用的變量(co_cellvars)
* ?     byte(PyUnicode 或 PyShortAscii)：文件名，它是從哪里加載的(co_filename)
* ?     byte(PyUnicode or PyShortAscii): 代碼本身的名字，默認是\<module\> (co_name)
* ?~?+3 byte(u32): 第一行數 (co_firstlineno)
* ?     byte(bytes)：行表，用 PyStringObject? (co_lnotab)

## PyTupleObject

* 0     byte: 0x29 (意思是:')')
* 01~04 byte(u32): 元組項數
* ?     byte(PyObject)：項目

## PyString 對象

* 如果我使用 ascii 以外的字符，它會變成 PyUnicode 嗎？
* “あ”、“??”和“α”是 PyUnicode(不再使用？)

* 0     byte：0x73(表示's')
* 1~4   byte：字符串長度
* 5~    byte：有效載荷

## PyUnicode 對象

* 0     byte：0x75(表示“u”)
* 1~4   byte：字符串長度
* 5~    byte：有效載荷

## PyShortAsciiObject

* 這叫短，但是即使超過100個字符，仍然會保持在短的狀態
* 或者更確切地說，沒有不短的 ascii(短數據類型嗎？)

* 0     byte：0xFA(表示“z”)
* 1~4   byte：字符串長度
* 5~    byte：有效載荷

## PyInternedObject

* 實習對象注冊在專用地圖中，可以與is進行比較
* 例如字符串，無論其長度如何，都可以在恒定時間內進行比較

* 0     byte：0x74(表示't')

## PyShortAsciiInternedObject

* 0     byte：0xDA(表示“Z”)
* 1~4   byte：字符串長度
* 5~    byte：有效載荷