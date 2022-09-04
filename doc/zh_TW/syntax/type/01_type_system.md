# Erg 類類型系統

下面是 Erg 類型系統的簡要說明。其他部分將介紹更多信息。

## 定義方法

Erg 的獨特之處在於，（常規）變量、函數（子程序）和類型（卡印度）的定義沒有太大的語法差異。所有這些都是根據常規變量和函數定義的語法定義的。


```erg
f i: Int = i + 1
f # <function f>
f(1) # 2
f.method self = ... # SyntaxError: cannot define a method to a subroutine

T I: Int = {...}
T # <kind 'T'>
T(1) # Type T(1)
T.method self = ...
D = Class {private = Int; .public = Int}
D # <class 'D'>
o1 = {private = 1; .public = 2} # o1はどのクラスにも屬さないオブジェクト
o2 = D.new {private = 1; .public = 2} # o2はDのインスタンス
o2 = D.new {.public = 2} # InitializationError: class 'D' requires attribute 'private'(: Int) but not defined
```

## 分類

Erg 中的所有對像都已輸入。最高類型是，它實現了<gtr=“16”/>，<gtr=“17”/>，<gtr=“18”/>等（它們不是請求方法，也不能覆蓋這些屬性）。 Erg 類型系統採用結構子類型（Structural subtyping，SST）。系統輸入的類型稱為“結構類型”（Structural type）。有三種結構類型：Attributive（屬性類型）/Refinement（篩子類型）/Algebraic（代數類型）。

|           | Record      | Enum       | Interval       | Union       | Intersection | Diff         |
| --------- | ----------- | ---------- | -------------- | ----------- | ------------ | ------------ |
| kind      | Attributive | Refinement | Refinement     | Algebraic   | Algebraic    | Algebraic    |
| generator | record      | set        | range operator | or operator | and operator | not operator |

也可以使用 Nominal subtyping（Nominal subtyping，NST），將 SST 類型轉換為 NST 類型稱為“類型記名”（Nominalization）。這種類型稱為“記名類型”（Nominal type）。在 Erg 中，記名類型為類和trait。如果只是一個類/任務，則通常指的是記錄類/記錄任務。

|     | Type           | Abstraction      | Subtyping procedure |
| --- | -------------- | ---------------- | ------------------- |
| NST | NominalType    | Trait            | Inheritance         |
| SST | StructuralType | Structural Trait | (Implicit)          |

表示整個記名類型的類型（）和整個結構類型的類型（<gtr=“20”/>）是整個類型的類型（<gtr=“21”/>）的子類型。

Erg 可以將參數（類型參數）傳遞給類型定義。具有類型參數的，<gtr=“23”/>等稱為多項卡印。它們本身不是類型，但通過應用參數成為類型。此外，沒有參數的<gtr=“24”/>或<gtr=“25”/>類型稱為簡單類型（標量類型）。

類型可以被視為一個集合，也存在包含關係。例如，包含<gtr=“27”/>和<gtr=“28”/>等，<gtr=“29”/>包含<gtr=“30”/>。所有類的上級類為<gtr=“31”/>，所有類型的下級類為<gtr=“32”/>。我們將在後面討論這一點。

## 型

像這樣的類型以<gtr=“34”/>為參數，返回<gtr=“35”/>類型，即<gtr=“36”/>類型的函數（理論上也稱為類型）。像<gtr=“37”/>這樣的類型被特別稱為多相類型，而<gtr=“38”/>本身被稱為 1 項卡印度。

參數和返回類型已知的函數類型將顯示為。如果要指定類型相同的 2 自變量函數整體，可以指定<gtr=“40”/>；如果要指定 N 自變量函數整體，可以指定<gtr=“41”/>。但是，由於<gtr=“42”/>類型沒有關於參數數量或類型的信息，因此調用時所有返回值都是<gtr=“43”/>類型。

類型應表示為<gtr=“45”/>，依此類推。此外，<gtr=“46”/>類型實例的名稱必須以<gtr=“47”/>結尾。

類型是一個函數/過程，它將其所屬的對象<gtr=“49”/>指定為第一個參數（作為引用）。對於依賴關係，你還可以在應用方法後指定自己的類型。這意味著你可以指定<gtr=“50”/>類型的方法，例如<gtr=“51”/>。

Erg 數組（Array）就是 Python 的列表。是包含三個<gtr=“53”/>類型對象的數組類。

> ：<gtr=“54”/>既是類型又是值，因此可以這樣使用。
>
> `` `erg
> Types = (Int, Str, Bool)
>
> for! Types, T =>
>     print! T
> # Int Str Bool
> a: Types = (1, "aaa", True)
> ```


```erg
pop|T, N|(l: [T; N]): ([T; N-1], T) =
    [...l, last] = l
    (l, last)

lpop|T, N|(l: [T; N]): (T, [T; N-1]) =
    [first, ...l] = l
    (first, l)
```

帶有的類型允許對象的內部結構重寫。例如，<gtr=“58”/>類是一個動態數組。要從<gtr=“59”/>類型對像生成<gtr=“60”/>類型對象，請使用一元運算符<gtr=“61”/>。


```erg
i: Int! = !1
i.update! i -> i + 1
assert i == 2
arr = [1, 2, 3]
arr.push! 4 # ImplError:
mut_arr = [1, 2, 3].into [Int; !3]
mut_arr.push! 4
assert mut_arr == [1, 2, 3, 4]
```

## 類型定義

類型定義如下。


```erg
Point2D = {.x = Int; .y = Int}
```

如果省略，例如<gtr=“62”/>，則它將成為類型中使用的私有變量。但這也是請求屬性。類型本身也有屬性，因為類型也是對象。這些屬性稱為類型屬性。類也稱為類屬性。

## 類型類、數據類型（等效）

如前所述，Erg 中的“類型”大致是指一組對象。以下是要求（中置運算符）的<gtr=“65”/>類型的定義。 <gtr=“66”/>是一個所謂的類型參數，它包含實現的類型（類），如<gtr=“67”/>和<gtr=“68”/>。在其他語言中，類型參數具有特殊的符號（通用、模板等），但在 Erg 中，類型參數的定義方式與常規參數的定義方式相同。類型參數也可以不是類型對象。例如，序列類型是的语法糖。如果類型實現被覆蓋，則用戶必須顯式選擇。


```erg
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
```

.是 Add.<gtr=“72”/>的縮寫。前綴運算符.<gtr=“73”/>是類型為<gtr=“74”/>的方法。


```erg
Num = Add and Sub and Mul and Eq
NumImpl = Patch Num
NumImpl.
    `+_`(self): Self = self
    ...
```

多相類型可以像函數一樣處理。單相化，例如（在許多情況下，即使未指定，也會使用實際參數進行推理）。


```erg
1 + 1
`_+_` 1, 1
Nat.`_+_` 1, 1
Int.`_+_` 1, 1
```

最上面的四行返回相同的結果（確切地說，最下面的行返回），但通常使用最上面的行。

```Ratio.`_+_`(1, 1)```とすると、エラーにはならず`2.0`が返ります。
これは、`Int <: Ratio`であるために`1`が`Ratio`にダウンキャストされるからです。
しかしこれはキャストされません。

```erg
i = 1
if i: # TypeError: i: Int cannot cast to Bool, use Int.is_zero() instead.
    log "a"
    log "b"
```

這是因為（<gtr=“78”/>，<gtr=“79”/>）。轉換到子類型通常需要驗證。

## 類型推理系統

Erg 採用靜態烤鴨打字，幾乎不需要明確指定類型。


```erg
f x, y = x + y
```

對於上面的代碼，將自動推斷具有的類型，即<gtr=“81”/>。 Erg 首先推論最小的類型。如果<gtr=“82”/>，則推論為<gtr=“83”/>；如果<gtr=“84”/>，則推論為<gtr=“85”/>。最小化後，類型將不斷增大，直到找到實現。對於<gtr=“86”/>，由於<gtr=“87”/>是具有<gtr=“88”/>實現的最小類型，因此將單相化為<gtr=“89”/>。 <gtr=“90”/>與<gtr=“91”/>不匹配，因此將單相化為<gtr=“92”/>。如果不是子類型或上類型關係，則從濃度（實例數）較低（如果是多相類型，則參數更少）開始嘗試。 <gtr=“93”/>和<gtr=“94”/>是作為<gtr=“95”/>和<gtr=“96”/>等部分類型的枚舉類型。枚舉類型等可以命名為請求/實現方法。在可以訪問該類型的命名空間中，滿足請求的對象可以使用實現方法。


```erg
Binary = Patch {0, 1}
Binary.
    # selfにはインスタンスが格納される。この例では0か1のどちらか。
    # selfを書き換えたい場合、型名、メソッド名に!を付けなければならない。
    is_zero(self) = match self:
        0 -> True
        1 -> False # _ -> Falseとしてもよい
    is_one(self) = not self.is_zero()
    to_bool(self) = match self:
        0 -> False
        1 -> True
```

以下代碼可能是（儘管<gtr=“98”/>是內置定義的）。如代碼中所示，下面是一個類型的示例，該類型實際上可以重寫<gtr=“99”/>。


```erg
Binary! = Patch {0, 1}!Binary!.
    switch! ref! self = match! self:
        0 => self = 1
        1 => self = 0

b = !1
b.switch!()
print! b # => 0
```

## 結構（未命名）


```erg
Binary = {0, 1}
```

在上面的代碼中，是元素的類型，其中<gtr=“101”/>和<gtr=“102”/>是元素的類型。也可以說是既有<gtr=“103”/>又有<gtr=“104”/>的<gtr=“105”/>類型的子類型。像這樣的對象本身就是一個類型，可以像上面那樣代入變量使用，也可以不代入變量使用。這種類型稱為結構類型。與類（記名型）對比，強調作為後者使用時，也稱為無名型。像這樣的結構類型稱為枚舉類型，其他類型包括區間類型和記錄類型。

### 類型同一性

不能像下面這樣指定。被解釋為指的是不同的東西。例如，<gtr=“109”/>和<gtr=“110”/>都是<gtr=“111”/>，但<gtr=“112”/>和<gtr=“113”/>不能相加。


```erg
add l: Add, r: Add =
    l + r # TypeError: there is no implementation of  `_+_`: |T, U <: Add| (T, U) -> <Failure>
```

此外，下面的和<gtr=“115”/>不能被視為同一類型。但是，類型<gtr=“116”/>被視為匹配。


```erg
... |R1; R2; O; A <: Add(R1, O); B <: Add(R2, O)|
```

<p align='center'>
    Previous | <a href='./02_basic.md'>Next</a>
</p>