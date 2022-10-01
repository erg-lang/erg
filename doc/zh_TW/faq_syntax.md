# Erg 部分設計的原因

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_syntax.md%26commit_hash%3D1b3d7827bb770459475e4102c6f5c43d8ad79ae4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_syntax.md&commit_hash=1b3d7827bb770459475e4102c6f5c43d8ad79ae4)

## Erg內存管理模型

在CPython後端中使用所有權 + Python內存管理模型(不過Erg代碼中的循環引用不會通過GC處理詳見[此處](syntax/18_ownership.md/#循環引用))

在Erg自己的虛擬機(Dyne)中使用所有權 + [Perceus](https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v1.pdf)內存管理模型，如果Erg代碼使用了Python API那麼這些Erg代碼使用跟踪垃圾回收內存管理模型

在LLVM, WASM後端使用所有權 + [Perceus](https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v1.pdf)內存管理模型

無論是什麼後端都不需要因為內存管理的不同對代碼進行任何更改

__注意__: Erg 引入所有權系統的動機不是像 Rust 那樣"不依賴 GC 的內存管理"。
Erg 所有權系統的目標是"可變狀態的本地化"。 Erg 有一個附屬於可變對象的所有權概念。
這是因為共享可變狀態容易出現錯誤，甚至違反類型安全(參見 [此處](./syntax/type/advanced/shared.md#共享參考))。這是一個判斷決定。

## 為什麼類型參數要大括號 || 而不是 <> 或 []?

這是因為 `<>` 和 `[]` 會導致語法衝突。

```python
# []版
id[T: Type] [t]: [T] = t
y = id[Int] # 這是一個功能嗎?
# <>版
id<T: Type> {t: T} = t
y = (id<Int, 1> 1) # 這是一個元組嗎?
# {}版
id{T: Type} {t: T} = t
y = id{Int} # 這是一個功能嗎?
# ||版
id|T: Type| t: T = t
y = id|Int| # OK
```

## {i=1} 的類型為 {i=Int}，但在 OCaml 等環境中為 {i：Int}。為什麼 Erg 採用前者的語法?

Erg 設計為將類型本身也視為值。

```python
A = [Int; 3]
assert A[2] == Int
T = (Int, Str)
assert T.1 == Str
D = {Int: Str}
assert D[Int] == Str
S = {.i = Int}
assert S.i == Int
```

## 你打算在 Erg 中實現宏嗎?

目前沒有。宏觀大致分為四個目的。第一個是編譯時計算。這在 Erg 中由編譯時函數負責。第二，代碼執行的延遲。這可以用 do 塊來代替。第三個是處理通用化，對此多相關數和全稱類型是比宏觀更好的解決方案。第四個是自動生成代碼，但這會造成可讀性的下降，所以我們不敢在 Erg 中實現。因此，宏的大部分功能都由 Erg 型系統承擔，因此沒有動力進行部署。

## 為什麼 Erg 沒有異常機制?

因為在許多情況下，使用 `Result` 類型進行錯誤處理是更好的解決方案。 `Result` 類型是相對較新的編程語言中使用的常見錯誤處理技術。

在 Erg 中，`?` 運算符使編寫無錯誤。

```python
read_file!() =
    f = open!("foo.txt")? # 如果失敗則立即返回錯誤，所以 f 是文件類型
    f.read_all!()

# 也可以使用 try 過程捕獲類似的異常
try!:
    do!
        s = read_file!()?
        print! s
    e =>
        # 發生錯誤時執行的塊
        print! e
        exit 1
```

在引入 Python 函數時，缺省情況下，所有函數都被視為包含異常，返回類型為。如果你知道不調度異常，請在<gtr="12"/>中指明。

此外，Erg 沒有引入異常機制的另一個原因是它計劃引入並行編程的功能。這是因為異常機制與並行執行不兼容(例如，如果並行執行導致多個異常，則很難處理)。

## Erg 似乎消除了 Python 被認為是壞做法的功能，但為什麼沒有取消繼承?

Python 的庫中有一些類設計為繼承，如果完全取消繼承，這些操作就會出現問題。然而，由於 Erg 的類默認為 final，並且原則上禁止多重和多層繼承，因此繼承的使用相對安全。

## 為什麼多相關數的子類型推理默認指向記名trait?

默認情況下，指向結構托盤會使類型指定變得複雜，並且可能會混合程序員的非預期行為。

```python
# 如果 T 是結構特徵的子類型...
# f: |T <: Structural Trait {.`_+_` = Self.(Self) -> Self; .`_-_` = Self.(Self) -> Self}| (T, T) -> T
f|T| x, y: T = x + y - x
# T 是名義特徵的子類型
# g: |T <: Add() and Sub()| (T, T) -> T
g|T| x, y: T = x + y - x
```

## Erg 是否實現了定義自己的運算符的功能?

A：沒有那個計劃。最重要的原因是，如果允許定義自己的運算符，就會出現如何處理組合順序的問題。可以定義自己的運算符的 Scala 和 Haskell 等都有不同的對應，但這可以看作是可能產生解釋差異的語法的證據。此外，獨立運算符還有一個缺點，那就是可能產生可讀性較低的代碼。

## 為什麼 Erg 取消了 += 這樣的擴展賦值運算符?

首先，Erg 中沒有變量可變性。換句話說，它不能被重新分配。一旦一個對象綁定到一個變量，它就會一直綁定到該變量，直到它超出範圍並被釋放。 Erg 中的可變性意味著對象可變性。一旦你知道了這一點，故事就很簡單了。例如，`i += 1` 表示 `i = i + 1`，但這樣的語法是非法的，因為變量沒有被重新分配。 Erg 的另一個設計原則是操作符不應該有副作用。 Python 大多是這樣，但是對於某些對象，例如 Dict，擴展賦值運算符會改變對象的內部狀態。這不是一個非常漂亮的設計。
這就是擴展賦值運算符完全過時的原因。

## 為什麼 Erg 在語法上特別對待有副作用的過程?

副作用的局部化是代碼維護的一個關鍵因素。

但是，確實也不是沒有方法可以不在語言上特殊對待副作用。例如，可以用代數效果(類型系統上的功能)替代過程。但這樣的合一併不總是正確的。例如，Haskell 沒有對字符串進行特殊處理，只是一個字符數組，但這種抽像是錯誤的。

什麼情況下，可以說合一化是錯的?一個指標是"是否會因其合一而難以看到錯誤信息"。 Erg 設計師發現，將副作用特殊處理會使錯誤消息更容易閱讀。

Erg 有一個強大的類型系統，但並不是所有的類型都決定了它。如果這樣做了，你的下場就跟 Java 試圖用類來控制一切一樣。
