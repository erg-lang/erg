# 副作用和程序

我們一直忽略了解釋“！”的含義，但現在它的含義終于要揭曉了。 這個 `!` 表示這個對象是一個帶有“副作用”的“過程”。 過程是具有副作用的函數。

```python
f x = print! x # EffectError: 不能為函數分配有副作用的對象
# 提示：將名稱更改為 'f!'
```

上面的代碼會導致編譯錯誤。 這是因為您在函數中使用了過程。 在這種情況下，您必須將其定義為過程。

```python
p! x = print! x
```

`p!`, `q!`, ... 是過程的典型變量名。
以這種方式定義的過程也不能在函數中使用，因此副作用是完全隔離的。

## 方法

函數和過程中的每一個都可以是方法。 函數式方法只能對`self`進行不可變引用，而程序性方法可以對`self`進行可變引用。
`self` 是一個特殊的參數，在方法的上下文中是指調用對象本身。 引用 `self` 不能分配給任何其他變量。

```python
C!.
    method ref self =
        x = self # 所有權錯誤：無法移出`self`
        x
```

程序方法也可以采取 `self` 的 [ownership](./18_ownership.md)。 從方法定義中刪除 `ref` 或 `ref!`

```python
n = 1
s = n.into(Str) # '1'
n # 值錯誤：n 被 .into 移動(第 2 行)
```

在任何給定時間，只有一種程序方法可以具有可變引用。 此外，在獲取可變引用時，不能從原始對象獲取更多可變引用。 從這個意義上說，`ref!` 會對`self` 產生副作用。

但是請注意，可以從可變引用創建(不可變/可變)引用。 這允許在程序方法中遞歸和 `print!` 的`self`。

```python
T -> T # OK (move)
T -> Ref T # OK (move)
T => Ref! T # OK (only once)
Ref T -> T # NG
Ref T -> Ref T # OK
Ref T => Ref!
T -> Ref T # NG
T -> Ref T # OK
T => Ref!
```

## 附錄：副作用的嚴格定義

代碼是否具有副作用的規則無法立即理解。
直到你能理解它們，我們建議你暫時把它們定義為函數，如果出現錯誤，添加`！`將它們視為過程。
但是，對于那些想了解該語言的確切規范的人，以下是對副作用的更詳細說明。

首先，必須聲明返回值的等價與 Erg 中的副作用無關。
有些過程對于任何給定的 `x` 都會導致 `p!(x) == p!(x)`(例如，總是返回 `None`)，并且有些函數會導致 `f(x) ！ = f(x)`。

前者的一個例子是`print!`，后者的一個例子是下面的函數。

```python
nan _ = Float.NaN
assert nan(1) ! = nan(1)
```

還有一些對象，例如類，等價確定本身是不可能的

```python
T = Structural {i = Int}
U = Structural {i = Int}
assert T == U

C = Class {i = Int}
D = Class {i = Int}
assert C == D # 類型錯誤：無法比較類
```

言歸正傳：Erg 中“副作用”的準確定義是

* 訪問可變的外部信息。

“外部”一般是指外部范圍； Erg 無法觸及的計算機資源和執行前/執行后的信息不包含在“外部”中。 “訪問”包括閱讀和寫作。

例如，考慮 `print!` 過程。 乍一看，`print!` 似乎沒有重寫任何變量。 但如果它是一個函數，它可以重寫外部變量，例如，使用如下代碼：

```python
camera = import "some_camera_module"
ocr = import "some_ocr_module"

n = 0
_ =
    f x = print x # 假設我們可以使用 print 作為函數
    f(3.141592)
cam = camera.new() # 攝像頭面向 PC 顯示器
image = cam.shot!()
n = ocr.read_num(image) # n = 3.141592
```

將“camera”模塊視為為特定相機產品提供 API 的外部庫，將“ocr”視為用于 OCR(光學字符識別)的庫。
直接的副作用是由 `cam.shot!()` 引起的，但顯然這些信息是從 `f` 泄露的。 因此，`print!` 本質上不可能是一個函數。

然而，在某些情況下，您可能希望臨時檢查函數中的值，而不想為此目的在相關函數中添加 `!`。 在這種情況下，可以使用 `log` 函數。
`log` 打印整個代碼執行后的值。 這樣，副作用就不會傳播。

```python
log "this will be printed after execution"
print! "this will be printed immediately"
# 這將立即打印
# 這將在執行后打印
```

如果沒有反饋給程序，或者換句話說，如果沒有外部對象可以使用內部信息，那么信息的“泄漏”是可以允許的。 只需要不“傳播”信息。

<p align='center'>
    <a href='./06_operator.md'>上一頁</a> | <a href='./08_procedure.md'>下一頁</a>
</p>
