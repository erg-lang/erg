# 提示

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tips.md%26commit_hash%3D157f51ae0e8cf3ceb45632b537ebe3560a5500b7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tips.md&commit_hash=157f51ae0e8cf3ceb45632b537ebe3560a5500b7)

## 想要更改顯示錯誤的語言

請為您的語言下載 Erg
但是，外部庫可能不支持多種語言

## 只想更改記錄的某些屬性

```python
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; rest; ...} = record
mut_record = {.height = !height; ...rest}
```

## 想要隱藏變量

使用 Erg 無法在相同范圍內進行遮蔽。 但是，如果范圍發生變化，您可以重新定義它們(這是一種稱為實例塊的語法)

````python
## 獲取一個 T!-type 對象，最后將它作為 T 類型賦值給一個變量
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
````

## 想以某種方式重用最終類(不可繼承的類)

您可以創建一個包裝類。 這就是所謂的構圖模式

```python
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## 想使用不是字符串的枚舉類型

可以定義其他語言中常見的傳統枚舉類型(代數數據類型)如下
如果您實現"單例"，則類和實例是相同的
此外，如果您使用 `Enum`，則選擇的類型會自動定義為重定向屬性

```python
Ok = Class Impl := Singleton
Err = Class Impl := Singleton
ErrWithInfo = Inherit {info = Str}
Status = Enum Ok, Err, ErrWithInfo
stat: Status = Status.new ErrWithInfo.new {info = "error caused by ..."}
match! stat:
    Status.Ok -> ...
    Status.Err -> ...
    Status.ErrWithInfo::{info} -> ...
```

```python
Status = Enum Ok, Err, ErrWithInfo
# 相當于
Status = Class Ok or Err or ErrWithInfo
Status.
    Ok = Ok
    Err = Err
    ErrWithInfo = ErrWithInfo
```

## 我想在1開頭枚舉

方法一: 

```python
arr = [...]
for! arr.iter().enumerate(start: 1), i =>
    ...
```

method 2:

```python
arr = [...]
for! arr.iter().zip(1...) , i =>
    ...
```

## 想要測試一個(白盒)非公共 API

`foo.er` 中的私有 API 可在 `foo.test.er` 模塊中特別訪問
`foo.test.er` 模塊無法導入，因此它保持隱藏狀態

```python
# foo.er
private x = ...
```

```python
# foo.test.er
foo = import "foo"

@Test
'testing private' x =
    ...
    y = foo::private x
    ...
```

## 想定義一個從外部只讀的(變量)屬性

您可以將屬性設為私有并定義一個 getter

```python
C = Class {v = Int!}
C::
    inc_v!(ref! self) = self::v.inc!()
    ...
C.
    get_v(ref self): Int = self::v.freeze()
    ...
```

## 希望在類型系統上識別參數名稱

您可以按記錄接收參數

```python
Point = {x = Int; y = Int}

norm: Point -> Int
norm({x: Int; y: Int}): Int = x**2 + y**2
assert norm({x = 1; y = 2}) == norm({y = 2; x = 1})
```

## 想要停止警告

Erg 中沒有停止警告的選項(這是設計使然)。 請重寫你的代碼
