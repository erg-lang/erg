# 類型擦除

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/erasure.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/erasure.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

類型擦除是將類型參數設置為 `_` 并故意丟棄其信息的過程。類型擦除是許多多態語言的特性，但在 Erg 的語法上下文中，將其稱為類型參數擦除更為準確

類型擦除的最常見示例是 `[T, _]`。數組在編譯時并不總是知道它們的長度。例如，引用命令行參數的 `sys.argv` 的類型為 `[Str, _]`。由于 Erg 的編譯器無法知道命令行參數的長度，因此必須放棄有關其長度的信息
然而，一個已經被類型擦除的類型變成了一個未被擦除的類型的父類型(例如`[T; N] <: [T; _]`)，所以它可以接受更多的對象
類型的對象`[T; N]` 當然可以使用 `[T; _]`，但使用后會刪除`N`信息。如果長度沒有改變，那么可以使用`[T; N]` 在簽名中。如果長度保持不變，則必須由簽名指示

```python
# 保證不改變數組長度的函數(例如，排序)
f: [T; N] -> [T; N] # 沒有的函數 (f: [T; N])
# 沒有的功能(例如過濾器)
g: [T; n] -> [T; _]
```

如果您在類型規范本身中使用 `_`，則類型將向上轉換為 `Object`
對于非類型類型參數(Int、Bool 等)，帶有 `_` 的參數將是未定義的

```python
i: _ # i: Object
[_; _] == [Object; _] == List
```

類型擦除與省略類型說明不同。一旦類型參數信息被刪除，除非您再次聲明它，否則它不會被返回

```python
implicit = (1..5).iter().map(i -> i * 2).to_arr()
explicit = (1..5).iter().map(i -> i * 2).into(List(Nat))
```

在 Rust 中，這對應于以下代碼:

```rust
let partial = (1..6).iter().map(|i| i * 2).collect::<Vec<_>>();
```

Erg 不允許部分省略類型，而是使用高階種類多態性

```python
# collect 是采用 Kind 的高階 Kind 方法
hk = (1..5).iter().map(i -> i * 2).collect(List)
hk: List(Int)
```
