# 解決補丁方法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/trait_method_resolving.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/trait_method_resolving.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

`Nat` 是零個或多個`Int`，`Int` 的子類型
`Nat` 在 Python 類層次結構中不存在。 我想知道 Erg 是如何解決這個補丁方法的?

```python
1.times do:
    log "hello world"
```

`.times` 是一種 `NatImpl` 補丁方法
由于`1`是`Int`的一個實例，首先通過跟蹤`Int`的MRO(方法解析順序)來搜索它
Erg 在 `Int` 的 MRO 中有 `Int`、`Object`。它來自 Python(Python 中的`int.__mro__ == [int, object]`)
`.times` 方法在它們中都不存在。現在讓我們探索那個子類型

~

整數顯然應該在其超類型中包含實數、復數甚至整數，但這一事實并沒有出現在 Python 兼容層中
然而，`1 in Complex` 和 `1 in Num` 在 Erg 中實際上是 `True`
至于`Complex`，即使是與`Int`沒有繼承關系的類，也被判斷為類型兼容。這到底是怎么回事?

~

一個對象有無數種它所屬的類型
但是我們真的只需要考慮帶有方法的類型，即帶有名稱的類型

Erg 編譯器有一個補丁類型的哈希圖，其中包含所有提供的方法及其實現
每次定義新類型時都會更新此表

```python
provided_method_table = {
    ...
    "foo": [Foo],
    ...
    ".times": [Nat, Foo],
    ...
}
```

具有 `.times` 方法的類型是 `Nat`、`Foo`。從這些中，找到與"{1}"類型匹配的一個
有兩種類型的符合性確定。它們是篩式判斷和記錄式判斷。這是通過篩子類型確定來完成的

##篩型確定

檢查候選類型是否與 `1` 的類型 `{1}` 兼容。與"{1}"兼容的篩子類型有"{0, 1}"、"0..9"等
有限元代數類型，例如 `0..1 或 3..4`、`-1..2 和 0..3`，在聲明為基本類型(即 {0, 1, 3, 4}`，`{0, 1, 2}`)
在這種情況下，`Nat` 是 `0.._ == {I: Int | I >= 0}`，所以 `{1}` 與 `Nat` 兼容

## 確定記錄類型

檢查候選類型是否與 `Int` 兼容，1 類
其他是`Int`的修復程序并且`Int`具有所有必需屬性的也是兼容的

~

所以`Nat`適合。但是，如果 `Foo` 也匹配，則由 `Nat` 和 `Foo` 之間的包含關系決定
即，選擇子類型方法
如果兩者之間沒有包含關系，則會發生編譯錯誤(這是一種安全措施，防止違背程序員的意圖執行方法)
要消除錯誤，您需要明確指定補丁

```python
o.method(x) -> P.method(o, x)
```

## 通用方法解析修補程序

像這樣定義一個補丁: 

```python
FnType T: Type = Patch T -> T
FnType.type = T
```

在 `FnType` 補丁下可以使用如下代碼。 我想知道這將如何解決

```python
assert (Int -> Int).type == Int
```

首先，`FnType(T)` 以下列格式注冊到`provided_method_table` 中

```python
provided_method_table = {
    ...
    "type": [FnType(T)],
    ...
}
```

`FnType(T)` 檢查匹配類型。 在這種情況下，`FnType(T)` 補丁類型是 `Type -> Type`
這匹配 `Int -> Int`。 如果合適，進行單態化和替換(取 `T -> T` 和 `Int -> Int`、`{T => Int}` 的差異)

```python
assert FnType(Int).type == Int
```