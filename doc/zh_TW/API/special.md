# 特殊形式

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/special.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/special.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

特殊形式是不能在 Erg 類型系統中表達的運算符、子程序(等等)。它被`包圍，但實際上無法捕獲
此外，為方便起見，還出現了"Pattern"、"Body"和"Conv"等類型，但不存在此類類型。它的含義也取決于上下文

## `=`(pat: Pattern, body: Body) -> NoneType

將 body 分配給 pat 作為變量。如果變量已存在于同一范圍內或與 pat 不匹配，則引發錯誤
它還用于記錄屬性定義和默認參數

```python
record = {i = 1; j = 2}
f(x: Int, y = 2) = ...
```

當主體是類型或函數時，`=` 具有特殊行為
左側的變量名嵌入到右側的對象中

```python
print! Class() # <class <lambda>>
print! x: Int -> x + 1 # <function <lambda>>
C = Class()
print! c # <class C>
f = x: Int -> x + 1
print! f # <function f>
g x: Int = x + 1
print! g # <function g>
K X: Int = Class(...)
print! K # <kind K>
L = X: Int -> Class(...)
print! L # <kind L>
```

`=` 運算符的返回值為"未定義"
函數中的多個賦值和 `=` 會導致語法錯誤

```python
i = j = 1 # SyntaxError: 不允許多次賦值
print!(x=1) # SyntaxError: cannot use `=` in function arguments
# 提示: 您的意思是關鍵字參數(`x: 1`)嗎?
if True, do:
    i = 0 # SyntaxError: 塊不能被賦值表達式終止
```

## `->`(pat: Pattern, body: Body) -> Func

生成匿名函數，函數類型

## `=>`(pat: Pattern, body: Body) -> Proc

生成匿名過程，過程類型

## `:`(subject, T)

確定主題是否與 T 匹配。如果它們不匹配，則拋出編譯錯誤

```python
a: Int
f x: Int, y: Int = x / y
```

也用于 `:` 應用樣式

```python
f x:
    y
    z
```

像`:`和`=`一樣，運算的結果是不確定的

```python
_ = x: Int # 語法錯誤: 
print!(x: Int) # 語法錯誤: 
```

## `.`(obj, attr)

讀取obj的屬性
`x.[y, z]` 將 x 的 y 和 z 屬性作為數組返回

## `|>`(obj, c: Callable)

執行`c(obj)`。`x + y |>.foo()` 與 `(x + y).foo()` 相同

### (x: Option T)`?` -> T | T 

后綴運算符。如果出現錯誤，請立即調用 `x.unwrap()` 和 `return`

## match(obj, ...lambdas: Lambda)

對于 obj，執行與模式匹配的 lambda

```python
match [1, 2, 3]:
  (l: Int) -> log "this is type of Int"
  [[a], b] -> log a, b
  [...a] -> log a
# (1, 2, 3)
```

## del(x: ...T) -> NoneType | T

刪除變量"x"。但是，無法刪除內置對象

```python
a = 1
del a # OK

del True # SyntaxError: cannot delete a built-in object
```

## do(body: Body) -> Func

生成一個不帶參數的匿名函數。`() ->` 的語法糖

## do!(body: Body) -> Proc

生成不帶參數的匿名過程。`() =>` 的語法糖

## `else`(l, r) -> Choice

創建一個由兩對組成的類元組結構，稱為 Choice 對象
`l, r` 被懶惰地評估。也就是說，只有在調用 .get_then 或 .get_else 時才會計算表達式

```python
choice = 1 else 2
assert choice.get_then() == 1
assert choice.get_else() == 2
assert True.then(choice) == 1
```

## 集合運算符

### `[]`(...objs)

從參數創建一個數組或從可選參數創建一個字典

### `{}`(...objs)

從參數創建一個集合

### `{}`(...fields: ((Field, Value); N))

生成記錄

### `{}`(layout, ...names, ...preds)

生成篩型，等級2型

### `...`

展開嵌套集合。它也可以用于模式匹配

```python
[x, ...y] = [1, 2, 3]
assert x == 1 and y == [2, 3]
assert [x, ...y] == [1, 2, 3]
assert [...y, x] == [2, 3, 1]
{x; ...yz} = {x = 1; y = 2; z = 3}
assert x == 1 and yz == {y = 2; z = 3}
assert {x; ...yz} == {x = 1; y = 2; z = 3}
```

## 虛擬運算符

用戶不能直接使用的運算符

### ref(x: T) -> Ref T | T

返回對對象的不可變引用

### ref!(x: T!) -> Ref!T! | T!

返回對可變對象的可變引用。