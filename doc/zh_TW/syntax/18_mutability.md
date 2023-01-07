# 可變性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/18_mutability.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/18_mutability.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

正如我們已經看到的，所有 Erg 變量都是不可變的。但是，Erg 對象具有可變性的概念
以下面的代碼為例

```python
a = [1, 2, 3]
a = a + [4, 5, 6]
print! a # [1, 2, 3, 4, 5, 6]
```

上面的代碼實際上不能被 Erg 執行。這是因為它不可重新分配

可以執行此代碼

```python
b = ![1, 2, 3]
b.concat! [4, 5, 6]
print! b # [1, 2, 3, 4, 5, 6]
```

`a, b` 的最終結果看起來一樣，但它們的含義卻大不相同
雖然 `a` 是表示 `Nat` 數組的變量，但第一行和第二行指向的對象是不同的。名稱`a`相同，但內容不同

```python
a = [1, 2, 3]
print! id! a # 0x000002A798DFE940
_a = a + [4, 5, 6]
print! id! _a # 0x000002A798DFE980
```

`id!` 過程返回對象駐留的內存地址

`b` 是一個 `Nat` "動態" 數組。對象的內容發生了變化，但變量指向的是同一個東西

```python
b = ![1, 2, 3]
print! id! b # 0x000002A798DFE220
b.concat! [4, 5, 6]
print! id! b # 0x000002A798DFE220
```

```python
i = !0
if! True. do!
    do! i.inc!() # or i.add!(1)
    do pass
print! i # 1
```

`!` 是一個特殊的運算符，稱為 __mutation 運算符__。它使不可變對象可變
標有"！"的對象的行為可以自定義

```python
Point = Class {.x = Int; .y = Int}

# 在這種情況下 .x 是可變的，而 .y 保持不變
Point! = Class {.x = Int!; .y = Int}
Point!.
    inc_x! ref!(self) = self.x.update! x -> x + 1

p = Point!.new {.x = !0; .y = 0}
p.inc_x!()
print! p.x # 1
```

## 常量

與變量不同，常量在所有范圍內都指向同一事物
常量使用 `=` 運算符聲明

```python
PI = 3.141592653589
match! x:
    PI => print! "this is pi"
```

常量在全局以下的所有范圍內都是相同的，并且不能被覆蓋。因此，它們不能被 ``=`` 重新定義。此限制允許它用于模式匹配
`True` 和 `False` 可以用于模式匹配的原因是因為它們是常量
此外，常量總是指向不可變對象。諸如 `Str!` 之類的類型不能是常量
所有內置類型都是常量，因為它們應該在編譯時確定。可以生成非常量的類型，但不能用于指定類型，只能像簡單記錄一樣使用。相反，類型是其內容在編譯時確定的記錄

## 變量、名稱、標識符、符號

讓我們理清一些與 Erg 中的變量相關的術語

變量是一種為對象賦予名稱以便可以重用(或指向該名稱)的機制
標識符是指定變量的語法元素
符號是表示名稱的語法元素、記號

只有非符號字符是符號，符號不稱為符號，盡管它們可以作為運算符的標識符
例如，`x` 是一個標識符和一個符號。`x.y` 也是一個標識符，但它不是一個符號。`x` 和 `y` 是符號
即使 `x` 沒有綁定到任何對象，`x` 仍然是一個符號和一個標識符，但它不會被稱為變量
`x.y` 形式的標識符稱為字段訪問器
`x[y]` 形式的標識符稱為下標訪問器

變量和標識符之間的區別在于，如果我們在 Erg 的語法理論意義上談論變量，則兩者實際上是相同的
在 C 中，類型和函數不能分配給變量； int 和 main 是標識符，而不是變量(嚴格來說可以賦值，但有限制)
然而，在Erg語中，"一切都是對象"。不僅函數和類型，甚至運算符都可以分配給變量

<p align='center'>
    <a href='./17_iterator.md'>上一頁</a> | <a href='./19_ownership.md'>下一頁</a>
</p>
