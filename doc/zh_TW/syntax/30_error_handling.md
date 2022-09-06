# 錯誤處理系統

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/30_error_handling.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/30_error_handling.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

主要使用Result類型。
在 Erg 中，如果您丟棄 Error 類型的對象(頂層不支持)，則會發生錯誤。

## 異常，與 Python 互操作

Erg 沒有異常機制(Exception)。 導入 Python 函數時

* 將返回值設置為 `T 或 Error` 類型
* `T or Panic` 類型(可能導致運行時錯誤)

有兩個選項，`pyimport` 默認為后者。 如果要作為前者導入，請使用
在 `pyimport` `exception_type` 中指定 `Error` (`exception_type: {Error, Panic}`)。

## 異常和結果類型

`Result` 類型表示可能是錯誤的值。 `Result` 的錯誤處理在幾個方面優于異常機制。
首先，從類型定義中可以看出子程序可能會報錯，實際使用時也很明顯。

```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

在上面的示例中，僅憑此代碼無法判斷哪個函數引發了異常。 即使回到函數定義，也很難判斷函數是否拋出異常。

```python
# Erg
try!:
    do!:
        x = foo!()?.bar()
        y = baz!()
        qux!()?
    e =>
        print! e
```

另一方面，在這個例子中，我們可以看到 `foo!` 和 `qux!` 會引發錯誤。
確切地說，`y` 也可能是 `Result` 類型，但您最終必須處理它才能使用里面的值。

使用 `Result` 類型的好處不止于此。 `Result` 類型也是線程安全的。 這意味著錯誤信息可以(輕松)在并行執行之間傳遞。

## 語境

由于 `Error`/`Result` 類型本身不會產生副作用，不像異常，它不能有發送位置(Context)等信息，但是如果使用 `.context` 方法，可以將信息放在 `錯誤`對象。 可以添加。 `.context` 方法是一種使用 `Error` 對象本身并創建新的 `Error` 對象的方法。 它們是可鏈接的，并且可以包含多個上下文。
```python
f() =
    todo() \
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

請注意，諸如 `.msg` 和 `.kind` 之類的 `Error` 屬性不是次要的，因此它們不是上下文，并且不能像最初創建時那樣被覆蓋。

## 堆棧跟蹤

`Result` 類型由于其方便性在其他語言中經常使用，但與異常機制相比，它的缺點是難以理解錯誤的來源。
因此，在 Erg 中，`Error` 對象具有名為 `.stack` 的屬性，并再現了類似偽異常機制的堆棧跟蹤。
`.stack` 是調用者對象的數組。 每次 Error 對象被`return`(包括通過`?`)時，它都會將它的調用子例程推送到`.stack`。
如果它是 `?`ed 或 `.unwrap`ed 在一個不可能 `return` 的上下文中，它會因為回溯而恐慌。

```python
f x =
    ...
    y = foo.try_some(x)?
    ...

g x =
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
# ...
# Foo.try_some, line 10, file "foo.er"
# 10 | y = foo.try_some(x)?
# module::f, line 23, file "foo.er"
# 23 | y = f(x)?
# module::g, line 40, file "foo.er"
# 40 | i = g(1)?
# Error: ...
```

## 恐慌

Erg 還有一種處理不可恢復錯誤的機制，稱為 __panicing__。
不可恢復的錯誤是由外部因素引起的錯誤，例如軟件/硬件故障、嚴重到無法繼續執行代碼的錯誤或程序員未預料到的錯誤。 等如果發生這種情況，程序將立即終止，因為程序員的努力無法恢復正常運行。 這被稱為“恐慌”。

恐慌是通過 `panic` 功能完成的。

```python
panic "something went wrong!"
```

<p align='center'>
    <a href='./29_decorator.md'>上一頁</a> | <a href='./31_pipeline.md'>下一頁</a>
</p>