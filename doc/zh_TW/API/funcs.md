# 功能

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/funcs.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/funcs.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

## 基本功能

### if|T, U|(cond: Bool, then: T, else: U) -> T or U

### map|T, U|(i: Iterable T, f: T -> U) -> Map U

請注意，參數的順序與 Python 相反

### log(x: Object, type: LogType = Info) -> None

在調試顯示中記錄"x"。執行完成后匯總并顯示日志
支持表情符號的終端根據"類型"添加前綴

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

### panic(msg: Str) -> Panic

顯示msg并停止
支持表情符號的終端有一個🚨前綴

### discard|T|(x: *T) -> NoneType

扔掉`x`。不使用返回值時使用。與 `Del` 不同，它不會使變量 `x` 不可訪問

```python
p! x =
    # q!應該返回一些不是None或()的值
    # 如果不需要，請使用`discard`
    discard q!(x)
    f x

discard True
assert True # OK
```

### import(path: Path) -> Module or CompilerPanic

導入一個模塊。如果找不到模塊，則引發編譯錯誤

### eval(code: Str) -> Object

將`code`作為代碼進行評估并返回

### classof(object: Object) -> Class

返回`object`的類
但是，由于無法比較類，如果要判斷實例，請使用`object in Class`而不是`classof(object) == Class`
編譯時確定的結構類型是通過`Typeof`獲得的

## Iterator, List生成系統

### repeat|T|(x: T) -> RepeatIterator T

```python
rep = repeat 1 # Repeater(1)
for! rep, i =>
    print! i
# 1 1 1 1 1 ...
```

### dup|T, N|(x: T, N: Nat) -> [T; N]

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

## 定數式関數

### Class

生成新類。 與`Inherit`不同，通過`Class`與基類型（第一個參數`Base`）無關，并且方法丟失。

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

### Inherit

繼承類可以直接使用父類(`Super`)的方法。可以在第二參數`Layout`中指定新的布局。
此時，必須`Super.Base:> Layout`

```python
@Inheritable
C = Class {i = Int}
D = Inherit C, {i = Int; j = Int} # C.Layout == {i = Int} :> {i = Int; j = Int}
E! = Inherit C, {i = Int!} # {i = Int} :> {i = Int!}
```

### Trait

創造一個新的trait。目前，只能指定記錄類型

### Typeof

返回參數類型。如果要獲取運行時類，請使用`classof`
如果您將其用于類型規范，則會出現警告

```python,compile_warn
x: Typeof i = ...
# TypeWarning: Typeof(i) == Int, please replace it
```

### Deprecated

作為解碼器使用。警告不推薦使用的類型或函數
