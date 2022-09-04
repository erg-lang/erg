# 類型變量，量化

類型變量是用於指定子例程參數類型的變量，並指示其類型是可選的（非單相）。首先，作為引入類型變量的動機，讓我們考慮返回輸入的函數。


```erg
id x: Int = x
```

雖然為類型定義了一個按原樣返回輸入的<gtr=“25”/>函數，但該函數顯然可以為任何類型定義。讓我們使用<gtr=“27”/>來表示最大的類。


```erg
id x: Object = x

i = id 1
s = id "foo"
b = id True
```

你現在可以接受任何類型，但有一個問題。返回類型將擴展為。如果輸入是<gtr=“29”/>類型，則返回<gtr=“30”/>類型；如果輸入是<gtr=“31”/>類型，則返回<gtr=“32”/>類型。


```erg
print! id 1 # <Object object>
id(1) + 1 # TypeError: cannot add `Object` and `Int`
```

若要確保輸入類型與返回類型相同，請使用。類型變量在<gtr=“33”/>（類型變量列表）中聲明。


```erg
id|T: Type| x: T = x
assert id(1) == 1
assert id("foo") == "foo"
assert id(True) == True
```

我們在函數中稱為。雖然有細微差別，但它們相當於其他語言中稱為通用的功能。然後，全稱量化函數稱為<gtr=“36”/>。定義多相關數就像為所有類型定義相同形式的函數一樣（由於 Erg 禁止重載，所以下面的代碼實際上是不可能寫的）。


```erg
id|T: Type| x: T = x
# pseudo code
# ==
id x: Int = x
id x: Str = x
id x: Bool = x
id x: Ratio = x
id x: NoneType = x
...
```

此外，類型變量用於指定類型，因此可以推斷為<gtr=“38”/>類型。因此，<gtr=“39”/>可以簡單地省略為<gtr=“40”/>。此外，如果你可以推理非類型對象，如<gtr=“41”/>（<gtr=“42”/>），則可以省略它。

另外，如果任何類型太大，也可以給出限制。約束也有好處，例如，子類型規範允許你使用特定的方法。


```erg
# T <: Add
# => TはAddのサブクラス
# => 加算ができる
add|T <: Add| l: T, r: T = l + r
```

在此示例中，必須是<gtr=“44”/>類型的子類，並且實際賦值的<gtr=“45”/>必須與<gtr=“46”/>類型相同。在這種情況下，<gtr=“47”/>是指<gtr=“48”/>或<gtr=“49”/>。因為沒有定義<gtr=“50”/>和<gtr=“51”/>的相加，所以彈。

也可以這樣打字。


```erg
f|
    Y, Z: Type
    X <: Add Y, O1
    O1 <: Add Z, O2
    O2 <: Add X, _
| x: X, y: Y, z: Z  =
    x + y + z + x
```

如果註釋列表變長，最好預先聲明。


```erg
f: |Y, Z: Type, X <: Add(Y, O1), O1 <: Add(Z, O2), O2 <: Add(X, O3)| (X, Y, Z) -> O3
f|X, Y, Z| x: X, y: Y, z: Z  =
    x + y + z + x
```

與許多具有通用語言的語言不同，所有聲明的類型變量必須在偽參數列表（部分）或其他類型變量的參數中使用。這是 Erg 語言設計提出的要求，即所有類型變量都可以從實際參數中推理。因此，不能推理的信息（如返回類型）是從實際參數傳遞的。 Erg 可以從實際參數中傳遞類型。


```erg
Iterator T = Trait {
    # 戻り値の型を引數から渡している
    # .collect: |K: Type -> Type| Self(T).({K}) -> K(T)
    .collect(self(T), K: Type -> Type): K(T) = ...
    ...
}

it = [1, 2, 3].iter().map i -> i + 1
it.collect(Array) # [2, 3, 4]
```

類型變量只能在之間聲明。但是，聲明之後，直到脫離範圍為止，可以在任意場所使用。


```erg
f|X|(x: X): () =
    y: X = x.clone()
    log X.__name__
    log X

f 1
# Int
# <class Int>
```

使用時也可以顯式單相化，如下所示。


```erg
f: Int -> Int = id|Int|
```

在這種情況下，指定的類型優先於實際參數類型（否則將導致實際參數類型錯誤的類型錯誤）。也就是說，如果實際傳遞的對象可以轉換為指定的類型，則會進行轉換，否則會導致編譯錯誤。


```erg
assert id(1) == 1
assert id|Int|(1) in Int
assert id|Ratio|(1) in Ratio
# キーワード引數も使える
assert id|T: Int|(1) == 1
id|Int|("str") # TypeError: id|Int| is type `Int -> Int` but got Str
```

當這個語法與內含符號擊球時，必須用括起來。


```erg
# {id|Int| x | x <- 1..10}だと{id | ...}だと解釈される
{(id|Int| x) | x <- 1..10}
```

不能聲明與已有類型同名的類型變量。這是因為類型變量都是常量。


```erg
I: Type
# ↓ invalid type variable, already exists
f|I: Type| ... = ...
```

## 方法定義中的類型參數

缺省情況下，左側的類型參數被視為綁定變量。


```erg
K(T: Type, N: Nat) = ...
K(T, N).
    foo(x) = ...
```

如果使用不同的類型變量名稱，則會發出警告。


```erg
K(T: Type, N: Nat) = ...
K(U, M). # Warning: K's type variable names are 'T' and 'N'
    foo(x) = ...
```

常量在定義後的所有命名空間中都是相同的，因此也不能用於類型變量名稱。


```erg
N = 1
K(N: Nat) = ... # NameError: N is already defined

L(M: Nat) = ...
# M == N == 1のときのみ定義される
L(N).
    foo(self, x) = ...
# 任意のM: Natに対して定義される
L(M).
    .bar(self, x) = ...
```

雖然不能為每個類型參數定義多個參數，但可以定義同名的方法，因為沒有指定類型參數的從屬類型（非原始卡印）和已指定類型參數的從屬類型（原始卡印）沒有關係。


```erg
K(I: Int) = ...
K.
    # Kは真の型(原子カインド)ではないので、メソッドを定義できない
    # これはメソッドではない(スタティックメソッドに近い)
    foo(x) = ...
K(0).
    foo(self, x): Nat = ...
```

## 全稱型

在上一章中定義的函數可以是任何類型。那麼，“函數本身的類型”是什麼呢？


```erg
print! classof(id) # |T: Type| T -> T
```

得到了的類型。這被稱為<gtr=“60”/>，相當於 ML 中以<gtr=“58”/>形式提供的類型，Haskell 中以<gtr=“59”/>形式提供的類型。為什麼要加上“閉合”這個形容詞，後面會講到。

封閉的全稱類型是有限制的，只能將子程序類型全稱化，即只能將其置於左側子句中。但這已經足夠了。在 Erg 中，子程序是最基本的控制結構，因此當“想要處理任意 X”時，即“想要能夠處理任意 X 的子程序”。所以，全稱型和多相關數型是一個意思。以後基本上把這種類型稱為多相關數類型。

與無名函數一樣，多相關數類型具有類型變量名稱的任意性，但它們都是等值的。


```erg
assert (|T: Type| T -> T) == (|U: Type| U -> U)
```

在 Lambda 計算中，當α等值時，等號成立。由於類型運算存在一些限制，因此始終可以確定等價性（除非考慮終止性）。

## 多相關數類型的部分類型

多相關數類型可以是任何函數類型。這意味著它與任何函數類型具有子類型關係。讓我們來詳細了解一下這種關係。

這樣的“類型變量在左邊定義並在右邊使用的類型”稱為<gtr=“63”/>。與此相對，<gtr=“62”/>等“類型變量在右邊定義和使用的類型”稱為<gtr=“64”/>。

打開了的全稱型，成為同形的全部的“真的型”的超級型。相反，封閉的全稱類型是所有相同類型的“真實類型”的子類型。


```erg
(|T: Type| T -> T) < (Int -> Int) < (T -> T)
```

最好記住關閉的小/打開的大。但怎麼會。為了更好地理解每一個實例。


```erg
# id: |T: Type| T -> T
id|T|(x: T): T = x

# iid: Int -> Int
iid(x: Int): Int = x

# 任意の関數をそのまま返す
id_arbitrary_fn|T|(f1: T -> T): (T -> T) = f
# id_arbitrary_fn(id) == id
# id_arbitrary_fn(iid) == iid

# 多相関數をそのまま返す
id_poly_fn(f2: (|T| T -> T)): (|T| T -> T) = f
# id_poly_fn(id) == id
id_poly_fn(iid) # TypeError

# Int型関數をそのまま返す
id_int_fn(f3: Int -> Int): (Int -> Int) = f
# id_int_fn(id) == id|Int|
# id_int_fn(iid) == iid
```

類型的<gtr=“66”/>可以賦給<gtr=“67”/>類型的參數<gtr=“68”/>，因此可以認為是<gtr=“69”/>。相反，<gtr=“70”/>類型的<gtr=“71”/>不能賦給<gtr=“72”/>類型的參數<gtr=“73”/>，但它是<gtr=“76”/>，因為它可以賦給<gtr=“74”/>類型的參數<gtr=“75”/>。因此，確實是<gtr=“77”/>。

## 全稱和依賴關係

依存型和全稱型（多相關數型）有什麼關係，有什麼不同呢？依賴類型是一種採用參數的類型，而全稱類型是一種為參數提供任意性的類型（在要全稱的子程序中）。

重要的是，封閉的全稱類型本身沒有類型參數。例如，多相關數類型是取多相關數<gtr=“80”/>的類型，其定義是封閉的。不能使用該類型參數<gtr=“79”/>定義方法等。

在 Erg 中，類型本身也是一個值，因此採用參數的類型（例如函數類型）也必須是一個依賴類型。也就是說，多相關數類型既是全稱類型，也是依存類型。


```erg
PolyFn = Patch(|T| T -> T)
PolyFn.
    type self = T # NameError: cannot find 'T'
DepFn T = Patch(T -> T)
DepFn.
    type self =
        log "by DepFn"
        T

assert (Int -> Int).type() == Int # by DepFn
assert DepFn(Int).type() == Int # by DepFn
```