# Tips

## 我想改變錯誤的顯示語言

請下載語言版本的 erg。但是，標準庫之外可能不提供多語言支持。

## 只想改變記錄的特定屬性


```erg
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; rest; ...} = record
mut_record = {.height = !height; ...rest}
```

## 我要陰影變量

Erg 不能在同一範圍內進行陰影。但是，如果作用域變了，就可以重新定義，所以最好使用即時塊。


```erg
# T!型オブジェクトを取得し、最終的にT型として変數へ代入
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
```

## 想辦法重用 final class（不可繼承類）

我們來做個說唱班。這就是所謂的合成模式。


```erg
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## 要使用非字符串枚舉類型

你可以定義其他語言中常見的傳統枚舉類型（代數數據類型），如下所示。當實現時，類和實例是等同的。此外，如果使用<gtr=“13”/>，則可選擇的類型將自動定義為重定向屬性。


```erg
Ok = Class Impl := Singleton
Err = Class Impl := Singleton
ErrWithInfo = Inherit {info = Str}
Status = Enum Ok, Err, ErrWithInfo
stat: Status = Status.cons(ErrWithInfo) {info = "error caused by ..."}
match! stat:
    Status.Ok -> ...
    Status.Err -> ...
    Status.ErrWithInfo::{info;} -> ...
```


```erg
Status = Enum Ok, Err, ErrWithInfo
# is equivalent to
Status = Class Ok or Err or ErrWithInfo
Status.
    Ok = Ok
    Err = Err
    ErrWithInfo = ErrWithInfo
```

## 一開始想要 enumerate

method 1:


```erg
arr = [...]
for! arr.iter().enumerate(start: 1), i =>
    ...
```

method 2:


```erg
arr = [...]
for! arr.iter().zip(1..), i =>
    ...
```

## 我想測試我的私有 API（白盒）

名為的模塊可以專門訪問<gtr=“15”/>的專用 API。 <gtr=“16”/>模塊不能導入，因此保持了隱藏性。


```erg
# foo.er
private x = ...
```


```erg
# foo.test.er
foo = import "foo"

@Test
'testing private' x =
    ...
    y = foo::private x
    ...
```

## 要定義外部只讀（可變）屬性

最好將屬性設為私有，然後定義 getta。


```erg
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## 要在類型系統上標識參數名稱

將參數作為記錄接收比較好。


```erg
Point = {x = Int; y = Int}

norm: Point -> Int
norm({x: Int; y: Int}): Int = x**2 + y**2
assert norm({x = 1; y = 2}) == norm({y = 2; x = 1})
```

## 我不想發出警告

沒有用於阻止 Erg 警告的選項（這是故意的設計）。重寫代碼。