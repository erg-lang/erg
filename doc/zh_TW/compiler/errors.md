# Erg Compiler Errors

## AssignError

嘗試重寫不可變變量時發生

## AttributeError

嘗試訪問不存在的屬性時發生

## PurityError

當您在不允許副作用的范圍內(函數、不可變類型等)編寫導致副作用的代碼時發生

## MoveError

嘗試訪問已移動的變量時發生

## BorrowError

在存在對對象的借用時嘗試獲取可變引用時發生

## CyclicError

當你有一個明顯不可阻擋的循環時發生

```python
i: Int = i

f(): Int = g()
g() = f()

h(): Int = module::h()

T = U
U = T
```

## BytecodeError

當加載的字節碼損壞時發生

## CompileSystemError

在編譯器內部發生錯誤時發生

## EnvironmentError

如果您在安裝期間沒有訪問權限，則會發生這種情況

## FeatureError

在檢測到未正式提供的實驗性功能時發生

## ImportError

## IndentationError

檢測到不良縮進時發生
派生自SyntaxError

## NameError

當您訪問不存在的變量時發生

## NotImplementedError

當您調用具有定義但沒有實現的 API 時發生
派生自 TypeError

## PatternError

當檢測到非法模式時發生
派生自SyntaxError

## SyntaxError

在檢測到錯誤語法時發生

## TabError

在使用制表符進行縮進/間距時發生
派生自SyntaxError

## TypeError

當對象類型不匹配時發生

## UnboundLocalError

在定義之前使用變量時發生
更準確地說，它發生在以前使用過在范圍內定義的變量時

```python
i = 0
f x =
    y = i + x
    i = 1
    y + i
```

在這段代碼中，`y = i + x` 中的 `i` 是一個未定義的變量
但是，常量可以在定義之前在另一個函數中調用

```python
f() = g()
g() = f()
```

## Erg Compiler Warnings

## SyntaxWarning

它在語法上很好，但是當我們檢測到冗余或不常見的代碼(不必要的 `()` 等)時就會發生這種情況

```python
if (True): # SyntaxWarning: unnecessary parentheses
    ...
```

## DeprecationWarning

在不推薦使用引用的對象時發生
(開發人員在生成此警告時應始終提供替代方法作為提示)

## FutureWarning

當您檢測到將來可能導致問題的代碼時發生
此警告是由版本兼容性問題(包括庫)以及語法和 API 的更改引起的

## ImportWarning
