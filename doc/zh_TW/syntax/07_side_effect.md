# 副作用和過程

到目前為止，我一直沒有解釋中<gtr=“11”/>的含義，現在我終於明白了它的含義。這個！直截了當地表示此對像是具有“副作用”的“過程”。過程對函數產生了一種稱為“副作用”的效果。


```erg
f x = print! x # EffectError: functions cannot be assigned objects with side effects
# hint: change the name to 'f!'
```

上面的代碼是編譯錯誤。因為你在函數中使用過程。在這種情況下，必須將其定義為過程。


```erg
p! x = print! x
```

，<gtr=“13”/>，...是代表過程的典型變量名稱。以這種方式定義的過程也不能在函數中使用，因此副作用是完全隔離的。

## 方法

每個函數和過程都有一個方法。函數方法僅保留的不變引用，過程方法保留<gtr=“15”/>的可變引用。 <gtr=“16”/>是一個特殊參數，在方法上下文中是指調用的對象本身。引用的<gtr=“17”/>不能指定給任何其他變量。


```erg
C.
    method ref self =
        x = self # OwnershipError: cannot move out 'self'
        x
```

該方法還可以剝奪的所有權。該方法的定義不包括<gtr=“19”/>或<gtr=“20”/>。


```erg
n = 1
s = n.into(Str) # '1'
n # ValueError: n was moved by .into (line 2)
```

始終只能有一個過程方法具有可變引用。此外，當可變參照被取走時，將無法從原始對象獲取參照。從這個意義上說，會對<gtr=“22”/>產生副作用。

但是，請注意，可以從可變參照生成（不變/可變）參照。這允許你在過程方法中遞歸或。


```erg
T -> T # OK (move)
T -> Ref T # OK
T => Ref! T # OK (only once)
Ref T -> T # NG
Ref T -> Ref T # OK
Ref T => Ref! T # NG
Ref! T -> T # NG
Ref! T -> Ref T # OK
Ref! T => Ref! T # OK
```

## Appendix：嚴格定義副作用

代碼有沒有副作用的規則並不是馬上就能理解的。在理解之前，建議先將其定義為函數，然後在出現錯誤時將其定義為過程。但是，對於那些想要掌握語言嚴格規範的人來說，下面我們會更詳細地介紹副作用。

首先，請注意，返回值的等價性與 Erg 中的副作用無關。對於任何，都有一個過程（例如，總是返回<gtr=“27”/>），也有一個函數是<gtr=“28”/>。

前一個示例是，後一個示例是以下函數。


```erg
nan _ = Float.NaN
assert nan(1) != nan(1)
```

也有一些對象無法進行等價判定，例如類或函數。


```erg
T = Structural {i = Int}
U = Structural {i = Int}
assert T == U

C = Class {i = Int}
D = Class {i = Int}
assert C == D # TypeError: cannot compare classes
```

回到正題上來。 “副作用”在 Erg 中的確切定義是，

* 訪問外部可變信息

中選擇所需的牆類型。外部通常是指外部範圍。 “外部”不包括 Erg 無法接觸的計算機資源或運行前/運行後信息。 “訪問”不僅包括寫入，還包括讀取。

以過程為例。 <gtr=“31”/>看似沒有重寫任何變量。但是，如果這是一個函數，那麼外部變量可以用這樣的代碼重寫。


```erg
camera = import "some_camera_module"
ocr = import "some_ocr_module"

n = 0
_ =
    f x = print x # 仮にprintを関數として使えたとします
    f(3.141592)
cam = camera.new() # カメラはPCのディスプレイを向いています
image = cam.shot!()
n = ocr.read_num(image) # n = 3.141592
```

模塊是為相機產品提供 API 的外部庫，<gtr=“33”/>是 OCR（光學字符識別）的庫。直接副作用是由<gtr=“34”/>引起的，但顯然，這些信息是從<gtr=“35”/>洩露的。因此，<gtr=“36”/>在性質上不能是函數。

然而，當你在函數中臨時檢查值時，你可能不希望將附加到相關函數中。在這種情況下，可以使用函數<gtr=“38”/>。 <gtr=“39”/>在執行整個代碼後顯示值。這不會傳播副作用。


```erg
log "this will be printed after execution"
print! "this will be printed immediately"
# this will be printed immediately
# this will be printed after execution
```

換句話說，如果程序沒有反饋，即任何外部對像都不能使用該信息，則信息的“洩露”本身可能是允許的。不被“傳播”就行了。

<p align='center'>
    <a href='./06_operator.md'>Previous</a> | <a href='./08_procedure.md'>Next</a>
</p>