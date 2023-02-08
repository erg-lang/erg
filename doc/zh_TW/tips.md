# 提示

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tips.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tips.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

## 想要更改顯示錯誤的語言

請為您的語言下載 Erg
但是，外部庫可能不支持多種語言

## 只想更改記錄的某些屬性

```python
record: {.name = Str; .age = Nat; .height = CentiMeter}
{height; *rest} = record
mut_record = {.height = !height; *rest}
```

## 想要隱藏變量

使用 Erg 無法在相同范圍內進行遮蔽。但是，如果范圍發生變化，您可以重新定義它們(這是一種稱為實例塊的語法)

````python
## 獲取一個 T!-type 對象，最后將它作為 T 類型賦值給一個變量
x: T =
    x: T! = foo()
    x.bar!()
    x.freeze()
````

## 想以某種方式重用最終類(不可繼承的類)

您可以創建一個包裝類。這就是所謂的構圖模式

```python
FinalWrapper = Class {inner = FinalClass}
FinalWrapper.
    method self =
        self::inner.method()
    ...
```

## 我想在1開頭枚舉

方法一:

```python
arr = [...]
for! arr.iter().enumerate(start := 1), i =>
    ...
```

method 2:

```python
arr = [...]
for! arr.iter().zip(1..) , i =>
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

## 在實現特征的方法時，會對未使用的變量發出警告

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

你可以使用`discard`或`_ = ...`

```python
T = Trait {.f = (Self, x: Int, s: Str) -> Int}

C = Class T
C|<: T|.
    f self, x, s =
        discard s # or _ = s
        ...
```

## 想要停止警告

Erg 中沒有停止警告的選項(這是設計使然)。請重寫你的代碼
