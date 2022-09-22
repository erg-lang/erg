# 所有權制度

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/18_ownership.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/18_ownership.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

由于 Erg 是一種使用 Python 作為宿主語言的語言，因此內存管理的方法取決于 Python 的實現。
但語義上 Erg 的內存管理與 Python 的不同。 一個顯著的區別在于所有權制度和禁止循環引用。

## 所有權

Erg 有一個受 Rust 啟發的所有權系統。
Rust 的所有權系統通常被認為是深奧的，但 Erg 的所有權系統被簡化為直觀。
在 Erg 中，__mutable objects__ 是擁有的，并且在所有權丟失后無法引用。

```python
v = [1, 2, 3].into [Int; !3]

push! vec, x =
    vec.push!(x)
    vec

# v ([1, 2, 3])的內容歸w所有
w = push! v, 4
print! v # 錯誤：v 被移動了
print!w # [1, 2, 3, 4]
```

例如，當一個對象被傳遞給一個子程序時，就會發生所有權轉移。
如果您想在贈送后仍然擁有所有權，則需要克隆、凍結或借用。
但是，如后所述，可以借用的情況有限。

## 復制

復制一個對象并轉移其所有權。 它通過將 `.clone` 方法應用于實際參數來做到這一點。
復制的對象與原始對象完全相同，但相互獨立，不受更改影響。

復制相當于 Python 的深拷貝，由于它完全重新創建相同的對象，因此計算和內存成本通常高于凍結和借用。
需要復制對象的子例程被稱為"參數消耗"子例程。

```python
capitalize s: Str!=
    s. capitalize!()
    s

s1 = !"hello"
s2 = capitalize s1.clone()
log s2, s1 # !"HELLO hello"
```

## 凍結

我們利用了不可變對象可以從多個位置引用的事實，并將可變對象轉換為不可變對象。
這稱為凍結。 例如，在從可變數組創建迭代器時會使用凍結。
由于您不能直接從可變數組創建迭代器，請將其轉換為不可變數組。
如果您不想破壞數組，請使用 [`.freeze_map` 方法](./type/18_mut.md)。

```python
# 計算迭代器產生的值的總和
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3]
x.push!(4)
i = x.iter() # 類型錯誤：[Int; !4] 沒有方法 `iter`
y = x.freeze()
i = y.iter()
assert sum(i) == 10
y # y 仍然可以被觸摸
```

## 借

借用比復制或凍結便宜。
可以在以下簡單情況下進行借款：

```python
peek_str ref(s: Str!) =
    log s

s = !"hello"
peek_str s
```

借來的值稱為原始對象的 __reference__。
您可以"轉租"對另一個子例程的引用，但您不能使用它，因為您只是借用它。

```python
steal_str ref(s: Str!) =
    # 由于日志函數只借用參數，所以可以轉租
    log s
    # 錯誤，因為丟棄函數消耗了參數
    discard s # OwnershipError: 不能消費借來的值
    # 提示：使用 `clone` 方法
```

```python
steal_str ref(s: Str!) =
    # 這也不好(=消耗右邊)
     x = s # OwnershipError: 不能消費借來的值
    x
```

Erg 的引用比 Rust 的更嚴格。 引用是語言中的一等對象，但不能顯式創建，它們只能指定為通過 `ref`/`ref!` 傳遞的參數。
這意味著您不能將引用填充到數組中或創建將引用作為屬性的類。

但是，這樣的限制是語言中的自然規范，一開始就沒有引用，而且它們并沒有那么不方便。

## 循環引用

Erg 旨在防止無意的內存泄漏，如果內存檢查器檢測到循環引用，則會發出錯誤。 在大多數情況下，這個錯誤可以通過弱引用 `Weak` 來解決。 但是，由于無法生成循環圖等具有循環結構的對象，因此我們計劃實現一個 API，可以將循環引用作為不安全操作生成。

<p align='center'>
    <a href='./17_mutability.md'>上一頁</a> | <a href='./19_visibility.md'>下一頁</a>
</p>
