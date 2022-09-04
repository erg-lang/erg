# cast

## 上投

Python 沒有 cast 的概念，因為它採用烤鴨打字的語言。不需要上播，基本上也沒有下播。但是，由於 Erg 是靜態輸入的，因此可能需要強制轉換。一個簡單的例子是。 Erg 的語言規範沒有定義<gtr=“6”/>（Int，Ratio），即 Int（ <：Add（Ratio，Ratio））的運算。這是因為<gtr=“7”/>將 1 上傳到 Ratio 的實例 1.0。

~~Erg 擴展字節碼將類型信息添加到 BINARY_ADD 中，其中類型信息為 Ratio-Ratio。在這種情況下，BINARY_ADD 指令將轉換 Int，因此不會插入指定轉換的特殊指令。因此，例如，如果在子類中覆蓋了某個方法，但將父項指定為類型，則會強制類型（type coercion）並在父項方法中執行（在編譯時進行名稱限定以引用父項方法）。編譯器只執行強制類型驗證和名稱限定。運行時不會強制轉換對象（當前）。可能會實現強制轉換指令以進行執行優化。 ~~


```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    # オーバーライドする際にはOverrideデコレータが必要
    @Override
    greet!() = print! "Hello from Child"

greet! p: Parent = p.greet!()

parent = Parent.new()
child = Child.new()

greet! parent # "Hello from Parent"
greet! child # "Hello from Parent"
```

此行為不會導致與 Python 的不兼容。 Python 最初不為變量指定類型，因此所有變量都以類型變量輸入。由於類型變量選擇最小匹配類型，因此如果 Erg 不指定類型，則會實現與 Python 相同的行為。


```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    greet!() = print! "Hello from Child"

greet! some = some.greet!()

parent = Parent.new()
child = Child.new()

greet! parent # "Hello from Parent"
greet! child # "Hello from Child"
```

對於具有繼承關係的類型，和<gtr=“9”/>是自動實現的，你也可以使用它們。


```erg
assert 1 == 1.0
assert Ratio.from(1) == 1.0
assert 1.into<Ratio>() == 1.0
```

## 下鑄

降播通常是不安全的，轉換方式也不是顯而易見的，而是通過實現來實現。


```erg
IntTryFromFloat = Patch Int
IntTryFromFloat.
    try_from r: Float =
        if r.ceil() == r:
            then: r.ceil()
            else: Error "conversion failed"
```