# 元組

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/11_tuple.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/11_tuple.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

元組類似于數組，但可以保存不同類型的對象
這樣的集合稱為不等集合。相比之下，同構集合包括數組、集合等

```python
t = (1, True, "a")
(i, b, s) = t
assert(i == 1 and b == True and s == "a")
```

元組`t`可以以`t.n`的形式檢索第n個元素； 請注意，與 Python 不同，它不是 `t[n]`
這是因為訪問元組元素更像是一個屬性(在編譯時檢查元素的存在，并且類型可以根據 `n` 改變)而不是方法(數組的 `[]` 是一種方法)

```python
assert t.0 == 1
assert t.1 == True
assert t.2 == "a"
```

括號 `()` 在不嵌套時是可選的

```python
t = 1, True, "a"
i, b, s = t
```

元組可以保存不同類型的對象，因此它們不能像數組一樣被迭代

```python
t: ({1}, {2}, {3}) = (1, 2, 3)
(1, 2, 3).iter().map(x -> x + 1) # 類型錯誤: 類型 ({1}, {2}, {3}) 沒有方法 `.iter()`
# 如果所有類型都相同，則可以像數組一樣用`(T; n)`表示，但這仍然不允許迭代
t: (Int; 3) = (1, 2, 3)
assert (Int; 3) == (Int, Int, Int)
```

但是，非同質集合(如元組)可以通過向上轉換、相交等方式轉換為同質集合(如數組)
這稱為均衡

```python
(Int, Bool, Str) can be [T; 3] where T :> Int, T :> Bool, T :> Str
```

```python
t: (Int, Bool, Str) = (1, True, "a") # 非同質
a: [Int or Bool or Str; 3] = [1, True, "a"] # 同質的
_a: [Show; 3] = [1, True, "a"] # 同質的
_a.iter().map(x -> log x) # OK
t.try_into([Show; 3])? .iter().map(x -> log x) # OK
```

## 單元

零元素的元組稱為 __unit__。一個單元是一個值，但也指它自己的類型

```python
unit = ()
(): ()
```

Unit是所有图元的超级类。

```python
() :> (Int; 0)
() :> (Str; 0)
() :> (Int, Str)
...
```

該對象的用途是用于沒有參數和沒有返回值的過程等。Erg 子例程必須有參數和返回值。但是，在某些情況下，例如過程，可能沒有有意義的參數或返回值，只有副作用。在這種情況下，我們將單位用作"無意義的正式值"

```python
p!() =
    # `print!` does not return a meaningful value
    print! "Hello, world!"
p!: () => () # The parameter part is part of the syntax, not a tuple
```

但是，在這種情況下，Python 傾向于使用"無"而不是單位
在 Erg 中，當您從一開始就確定操作不會返回有意義的值(例如在過程中)時，您應該使用 `()`，并且當操作可能失敗并且您可能會返回 `None` 將一無所獲，例如在檢索元素時

<p align='center'>
    <a href='./10_array.md'>上一頁</a> | <a href='./12_dict.md'>下一頁</a>
</p>
