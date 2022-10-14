# 類型變量，量化類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/15_quantified.md%26commit_hash%3D14657486719a134f494e107774ac8f9d5a63f083)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/15_quantified.md&commit_hash=14657486719a134f494e107774ac8f9d5a63f083)

類型變量是用于例如指定子程序參數類型的變量，它的類型是任意的(不是單態的)
首先，作為引入類型變量的動機，考慮 `id` 函數，它按原樣返回輸入

```python
id x: Int = x
```

返回輸入的"id"函數是為"Int"類型定義的，但這個函數顯然可以為任何類型定義
讓我們使用 `Object` 來表示最大的類

```python
id x: Object = x

i = id 1
s = id "foo"
b = id True
```

當然，它現在接受任意類型，但有一個問題: 返回類型被擴展為 `Object`。 返回類型擴展為 `Object`
如果輸入是"Int"類型，我想查看返回類型"Int"，如果輸入是"Str"類型，我想查看"Str"

```python
print! id 1 # <Object object>
id(1) + 1 # 類型錯誤: 無法添加 `Object` 和 `Int
```

要確保輸入的類型與返回值的類型相同，請使用 __type 變量__
類型變量在`||`(類型變量列表)中聲明

```python
id|T: Type| x: T = x
assert id(1) == 1
assert id("foo") == "foo"
assert id(True) == True
```

這稱為函數的 __universal quantification(泛化)__。 有細微的差別，但它對應于其他語言中稱為泛型的函數。 泛化函數稱為__多態函數__
定義一個多態函數就像為所有類型定義一個相同形式的函數(Erg 禁止重載，所以下面的代碼真的不能寫)

```python
id|T: Type| x: T = x
# 偽代碼
id x: Int = x
id x: Str = x
id x: Bool = x
id x: Ratio = x
id x: NoneType = x
...
```

此外，類型變量"T"可以推斷為"Type"類型，因為它在類型規范中使用。 所以 `|T: Type|` 可以簡單地縮寫為 `|T|`
你也可以省略`|T, N| 腳; N]` 如果可以推斷它不是類型對象(`T: Type, N: Nat`)

如果類型對于任意類型來說太大，您也可以提供約束
約束也有優勢，例如，子類型規范允許使用某些方法

```python
# T <: Add
# => T 是 Add 的子類
# => 可以做加法
add|T <: Add| l: T, r: T = l + r
```

在本例中，`T` 必須是`Add` 類型的子類，并且要分配的`l` 和`r` 的實際類型必須相同
在這種情況下，"T"由"Int"、"Ratio"等滿足。因此，例如，"Int"和"Str"的添加沒有定義，因此被拒絕

您也可以像這樣鍵入它

```python
f|
    Y, Z: Type
    X <: Add Y, O1
    O1 <: Add Z, O2
    O2 <: Add X, _
| x: X, y: Y, z: Z =
    x + y + z + x
```

如果注釋列表很長，您可能需要預先聲明它

```python
f: |Y, Z: Type, X <: Add(Y, O1), O1 <: Add(Z, O2), O2 <: Add(X, O3)| (X, Y, Z) -> O3
f|X, Y, Z| x: X, y: Y, z: Z =
    x + y + z + x
```

與許多具有泛型的語言不同，所有聲明的類型變量都必須在臨時參數列表(`x: X, y: Y, z: Z` 部分)或其他類型變量的參數中使用
這是 Erg 語言設計的一個要求，即所有類型變量都可以從真實參數中推斷出來
因此，無法推斷的信息，例如返回類型，是從真實參數傳遞的； Erg 允許從實參傳遞類型

```python
Iterator T = Trait {
    # 從參數傳遞返回類型
    # .collect: |K: Type -> Type| Self(T). ({K}) -> K(T)
    .collect(self, K: Type -> Type): K(T) = ...
    ...
}

it = [1, 2, 3].iter().map i -> i + 1
it.collect(Array) # [2, 3, 4].
```

類型變量只能在 `||` 期間聲明。 但是，一旦聲明，它們就可以在任何地方使用，直到它們退出作用域

```python
f|X|(x: X): () =
    y: X = x.clone()
    log X.__name__
    log X

f 1
# Int
# <class Int>
```

您也可以在使用時明確單相如下

```python
f: Int -> Int = id|Int|
```

在這種情況下，指定的類型優先于實際參數的類型(匹配失敗將導致類型錯誤，即實際參數的類型錯誤)
即如果傳遞的實際對象可以轉換為指定的類型，則進行轉換； 否則會導致編譯錯誤

```python
assert id(1) == 1
assert id|Int|(1) in Int
assert id|Ratio|(1) in Ratio
# 你也可以使用關鍵字參數
assert id|T: Int|(1) == 1
id|Int|("str") # 類型錯誤: id|Int| is type `Int -> Int`但得到了 Str
```

當此語法與理解相沖突時，您需要將其括在 `()` 中

```python
# {id|Int| x | x <- 1..10} 將被解釋為 {id | ...}
{(id|Int| x) | x <- 1..10}
```

不能使用與已存在的類型相同的名稱來聲明類型變量。 這是因為所有類型變量都是常量

```python
I: Type
# ↓ 無效類型變量，已經存在
f|I: Type| ... = ...
```

## 在方法定義中輸入參數

默認情況下，左側的類型參數被視為綁定變量

```python
K(T: Type, N: Nat) = ...
K(T, N).
    foo(x) = ...
```

使用另一個類型變量名稱將導致警告

```python
K(T: Type, N: Nat) = ...
K(U, M). # 警告: K 的類型變量名是 'T' 和 'N'
    foo(x) = ...
```

自定義以來，所有命名空間中的常量都是相同的，因此它們當然不能用于類型變量名稱

```python
N = 1
K(N: Nat) = ... # 名稱錯誤: N 已定義

L(M: Nat) = ...
# 僅當 M == N == 1 時才定義
L(N).
    foo(self, x) = ...
# 為任何定義 M: Nat
L(M).
    .bar(self, x) = ...
```

每個類型參數不能有多個定義，但可以定義具有相同名稱的方法，因為未分配類型參數的依賴類型(非原始類型)和分配的依賴類型(原始類型)之間沒有關系 )

```python
K(I: Int) = ...
K.
    # K 不是真正的類型(atomic Kind)，所以我們不能定義方法
    # 這不是方法(更像是靜態方法)
    foo(x) = ...
K(0).
    foo(self, x): Nat = ...
```

## 所有對稱類型

上一節中定義的 `id` 函數是一個可以是任何類型的函數。 那么 `id` 函數本身的類型是什么?

```python
print! classof(id) # |T: Type| T -> T
```

我們得到一個類型`|T: Type| T -> T`。 這稱為一個 __封閉的全稱量化類型/全稱類型__，即`['a. ...]'` 在 ML 和 `forall t. ...` 在 Haskell 中。 為什么使用形容詞"關閉"將在下面討論

封閉的全稱量化類型有一個限制: 只有子程序類型可以被通用量化，即只有子程序類型可以放在左子句中。 但這已經足夠了，因為子程序是 Erg 中最基本的控制結構，所以當我們說"我要處理任意 X"時，即我想要一個可以處理任意 X 的子程序。所以，量化類型具有相同的含義 作為多態函數類型。 從現在開始，這種類型基本上被稱為多態函數類型

與匿名函數一樣，多態類型具有任意類型變量名稱，但它們都具有相同的值

```python
assert (|T: Type| T -> T) == (|U: Type| U -> U)
```

當存在 alpha 等價時，等式得到滿足，就像在 lambda 演算中一樣。 由于對類型的操作有一些限制，所以總是可以確定等價的(如果我們不考慮 stoppage 屬性)

## 多態函數類型的子類型化

多態函數類型可以是任何函數類型。 這意味著與任何函數類型都存在子類型關系。 讓我們詳細看看這種關系

類型變量在左側定義并在右側使用的類型，例如 `OpenFn T: Type = T -> T`，稱為 __open 通用類型__
相反，在右側定義和使用類型變量的類型，例如 `ClosedFn = |T: Type| T -> T`，被稱為 __封閉的通用類型__

開放通用類型是所有同構"真"類型的超類型。 相反，封閉的通用類型是所有同構真類型的子類型

```python
(|T: Type| T -> T) < (Int -> Int) < (T -> T)
```

您可能還記得封閉的較小/開放的較大
但為什么會這樣呢? 為了更好地理解，讓我們考慮每個實例

```python
# id: |T: Type| T -> T
id|T|(x: T): T = x

# iid: Int -> Int
iid(x: Int): Int = x

# 按原樣返回任意函數
id_arbitrary_fn|T|(f1: T -> T): (T -> T) = f
# id_arbitrary_fn(id) == id
# id_arbitrary_fn(iid) == iid

# return the poly correlation number as it is
id_poly_fn(f2: (|T| T -> T)): (|T| T -> T) = f
# id_poly_fn(id) == id
id_poly_fn(iid) # 類型錯誤

# 按原樣返回 Int 類型函數
id_int_fn(f3: Int -> Int): (Int -> Int) = f
# id_int_fn(id) == id|Int|
# id_int_fn(iid) == iid
```

由于 `id` 是 `|T: Type| 類型T -> T`，可以賦值給`Int-> Int`類型的參數`f3`，我們可以考慮`(|T| T -> T) < (Int -> Int)`
反之，`Int -> Int`類型的`iid`不能賦值給`(|T| T -> T)`類型的參數`f2`，但可以賦值給`(|T| T -> T)`的參數`f1`輸入 `T -> T`，所以 `(Int -> Int) < (T -> T)`
因此，確實是`(|T| T -> T) < (Int -> Int) < (T -> T)`

## 量化類型和依賴類型

依賴類型和量化類型(多態函數類型)之間有什么關系，它們之間有什么區別?
我們可以說依賴類型是一種接受參數的類型，而量化類型是一種賦予參數任意性的類型

重要的一點是封閉的多態類型本身沒有類型參數。例如，多態函數類型`|T| T -> T` 是一個接受多態函數 __only__ 的類型，它的定義是封閉的。您不能使用其類型參數`T`來定義方法等

在 Erg 中，類型本身也是一個值，因此帶參數的類型(例如函數類型)可能是依賴類型。換句話說，多態函數類型既是量化類型又是依賴類型

```python
PolyFn = Patch(|T| T -> T)
PolyFn.
    type self = T # 名稱錯誤: 找不到"T"
DepFn T = Patch(T -> T)
DepFn.
    type self =
        log "by DepFn"
        T

assert (Int -> Int).type() == Int # 由 DepFn
assert DepFn(Int).type() == Int # 由 DepFn
```
