# 幻影類

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/phantom.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/phantom.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

幻像類型是標記Trait，其存在僅用于向編譯器提供注釋
作為幻像類型的一種用法，讓我們看一下列表的結構

```python
Nil = Class()
List T, 0 = Inherit Nil
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

此代碼導致錯誤

```python
3 | List T, 0 = Inherit Nil
                        ^^^
類型構造錯誤: 由于Nil沒有參數T，所以無法用Nil構造List(T, 0)
提示: 使用 'Phantom' trait消耗 T
```

此錯誤是在使用 `List(_, 0).new Nil.new()` 時無法推斷 `T` 的抱怨
在這種情況下，無論 `T` 類型是什么，它都必須在右側使用。大小為零的類型(例如長度為零的元組)很方便，因為它沒有運行時開銷
```python
Nil T = Class((T; 0))
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

此代碼通過編譯。但是理解意圖有點棘手，除非類型參數是類型，否則不能使用它

在這種情況下，幻影類型正是您所需要的。幻像類型是大小為 0 的廣義類型

```python
Nil T = Class(Impl := Phantom T)
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}

nil = Nil(Int).new()
assert nil.__size__ == 0
```

`Phantom` 擁有`T` 類型。但實際上 `Phantom T` 類型的大小是 0 并且不包含 `T` 類型的對象

此外，`Phantom` 可以使用除其類型之外的任意類型參數。在下面的示例中，`Phantom` 包含一個名為 `State` 的類型參數，它是 `Str` 的子類型對象
同樣，`State` 是一個假的類型變量，不會出現在對象的實體中

```python
VM! State: {"stopped", "running"}! = Class(... State)
VM!("stopped").
    start ref! self("stopped" ~> "running") =
        self.do_something!()
        self::set_phantom!("running"))
```

`state` 是通過 `update_phantom!` 或 `set_phantom!` 方法更新的
這是標準補丁為`Phantom!`(`Phantom`的變量版本)提供的方法，其用法與變量`update!`和`set!`相同。