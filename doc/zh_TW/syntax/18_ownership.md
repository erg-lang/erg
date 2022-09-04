# 所有權制度

由於 Erg 是以 Python 為主機語言的語言，因此管理內存的方式依賴於 Python 的處理系統。然而，從語義上講，Erg 的內存管理與 Python 的內存管理不同。顯著的區別體現在所有權制度和禁止循環引用。

## 所有權

Erg 有一個所有權系統受到Rust的影響。 Rust 的所有權系統通常被稱為晦澀難懂，但 Erg 的所有權系統被簡化為直觀。 Erg 擁有的所有權，一旦失去所有權，就無法查看該對象。


```erg
v = [1, 2, 3].into [Int; !3]

push! vec, x =
    vec.push!(x)
    vec

# vの中身([1, 2, 3])の所有権はwに移る
w = push! v, 4
print! v # error: v was moved
print! w # [1, 2, 3, 4]
```

例如，在將對像傳遞到子例程時會發生所有權移動。如果你希望在傳遞後仍擁有所有權，則必須複製（cloning）、凍結（freeze）或借用（borrowing）。但是，如下文所述，可以藉用的場合有限。

## 複製

複製對象並轉移其所有權。通過將方法應用於實際參數來完成此操作。複製的對象與原始對象完全相同，但它們彼此獨立，不受更改的影響。

複製相當於 Python 的深度副本，因為要重新創建整個相同的對象，所以與凍結和借用相比，通常計算和內存成本更高。需要復制對象的子例程稱為使用參數子例程。


```erg
capitalize s: Str! =
    s.capitalize!()
    s

s1 = !"hello"
s2 = capitalize s1.clone()
log s2, s1 # !"HELLO hello"
```

## 凍結

利用可變對象可以從多個位置引用，將可變對象轉換為不變對象。這叫凍結。凍結可用於創建可變陣列的迭代器。變量數組無法直接創建迭代器，因此將其轉換為不變數組。如果不想破壞數組，請使用 [方法] （./type/mut.md）等。


```erg
# イテレータが出す値の合計を計算する
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3]
x.push!(4)
i = x.iter() # TypeError: [Int; !4] has no method `iter`
y = x.freeze()
i = y.iter()
assert sum(i) == 10
y # この後もyは觸れられる
```

## 借用

借用比複製和凍結成本更低。在以下簡單情況下，可以藉用。


```erg
peek_str ref(s: Str!) =
    log s

s = !"hello"
peek_str s
```

對於原始對象，借用的值稱為。你可以將引用“轉借”給另一個子例程，但不能消費，因為它只是藉用。


```erg
steal_str ref(s: Str!) =
    # log関數は引數を借用するだけなので、又貸しできる
    log s
    # discard関數は引數を消費するので、エラー
    discard s # OwnershipError: cannot consume a borrowed value
    # hint: use `clone` method
```


```erg
steal_str ref(s: Str!) =
    # これもダメ(=は右辺を消費する)
    x = s # OwnershipError: cannot consume a borrowed value
    x
```

Erg 引用比 Rust 具有更強的約束。引用是第一級語言對象，但不能顯式生成，只能通過/<gtr=“12”/>指定實際參數的傳遞方式。這意味著你不能將引用合併到數組中，也不能創建以引用為屬性的類。

儘管如此，這種限制在沒有參照的語言中本來就是理所當然的規範，並沒有那麼不方便。

## 循環引用

Erg 的設計目的是防止意外發生內存洩漏，當內存檢查器檢測到循環引用時，它會發出錯誤消息。在大多數情況下，可以使用弱引用來解決此錯誤。但是，由於這無法生成具有循環結構的對象（如循環圖），因此我們計劃實現一個 API，該 API 可以生成循環引用作為 unsafe 操作。

<p align='center'>
    <a href='./17_mutability.md'>Previous</a> | <a href='./19_visibility.md'>Next</a>
</p>