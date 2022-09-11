# Python 字節碼規範

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/python/bytecode_specification.md%26commit_hash%3D9f6a4a43fcf7e4f58cabe6e5a7546820fd9f5ff4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/python/bytecode_specification.md&commit_hash=9f6a4a43fcf7e4f58cabe6e5a7546820fd9f5ff4)


## 格式

* 0~3   字節(u32)：幻數(詳見common/bytecode.rs)
* 4~7   字節(u32): 0 padding
* 8~12  字節(u32): 時間戳
* 13~   字節(PyCodeObject): 代碼對象

## PyCode 對象

* 0     字節(u8): '0xe3' (前綴，這意味著代碼的'c')
* 01~04 字節(u32): args個數(co_argcount)
* 05~08 字節(u32): position-only args 的數量 (co_posonlyargcount)
* 09~12 字節(u32)：僅關鍵字參數的數量(co_kwonlyargcount)
* 13~16 字節(u32): 本地數 (co_nlocals)
* 17~20 字節(u32): 棧大小(co_stacksize)
* 21~24 字節(u32)：標誌(co_flags)()
* ?     字節：字節碼指令，以'0x53'、'0x0'結尾(83, 0)：RETURN_VALUE(co_code)
* ?     字節(PyTuple)：代碼中使用的常量(co_consts)
* ?     字節(PyTuple)：代碼中使用的名稱(co_names)
* ?     字節(PyTuple)：代碼中定義的變量名，包括params (PyTuple) (co_varnames)
* ?     字節(PyTuple)：從外部範圍捕獲的變量(co_freevars)
* ?     字節(PyTuple)：內部閉包中使用的變量(co_cellvars)
* ?     字節(PyUnicode 或 PyShortAscii)：文件名，它是從哪裡加載的(co_filename)
* ?     字節(PyUnicode or PyShortAscii): 代碼本身的名字，默認是\<module\> (co_name)
* ?~?+3 字節(u32): 第一行數 (co_firstlineno)
* ?     字節(bytes)：行表，用 PyStringObject? (co_lnotab)

## Py 元組對象

* 0     字節: 0x29 (意思是:`)`)
* 01~04 字節(u32): 元組項數
* ?     字節(PyObject)：項目

## PyString 對象

* 如果我使用 ascii 以外的字符，它會變成 PyUnicode 嗎?
* "あ"、"𠮷"和"α"是 PyUnicode(不再使用?)

* 0     字節：0x73(表示`s`)
* 1~4   字節：字符串長度
* 5~    字節：有效載荷

## PyUnicode 對象

* 0     字節：0x75(表示`u`)
* 1~4   字節：字符串長度
* 5~    字節：有效載荷

## PyShortAscii 對象

* 這叫短，但是即使超過100個字符，仍然會保持在短的狀態
* 或者更確切地說，沒有不短的 ascii(短數據類型嗎?)

* 0     字節：0xFA(表示`z`)
* 1~4   字節：字符串長度
* 5~    字節：有效載荷

## PyInterned 對象

* 實習對象註冊在專用地圖中，可以與is進行比較
* 例如字符串，無論其長度如何，都可以在恆定時間內進行比較

* 0     字節：0x74(表示`t`)

## PyShortAsciiInterned 對象

* 0     字節：0xDA(表示`Z`)
* 1~4   字節：字符串長度
* 5~    字節：有效載荷