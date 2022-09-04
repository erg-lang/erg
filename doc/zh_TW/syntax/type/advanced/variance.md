# 退化（variance）

Erg 可以進行多相型的分型，但是有一部分必須注意的地方。

首先考慮通常的多相型的包含關係。一般情況下，存在容器和代入的類型<gtr=“8”/>，當<gtr=“9”/>時，為<gtr=“10”/>。例如，<gtr=“11”/>。因此，用<gtr=“12”/>定義的方法，也可以使用<gtr=“13”/>。

考慮典型的多相型型。請注意，這一次不考慮元素的數量，因此不是。那麼，在<gtr=“16”/>型中存在<gtr=“17”/>和<gtr=“18”/>的方法，分別表示要素的追加、取出。套路是這樣的。

Array.push!: Self(T).(T) => NoneTypeArray.pop!: Self(T).() => T

我們可以直觀地了解到，

* 當時<gtr=“20”/>OK（將<gtr=“21”/>上傳至<gtr=“22”/>即可）
* 當時<gtr=“24”/>為 NG
* 是 NG
* 好的

是。這在類型系統上

* (Self(Object).(Object) => NoneType) < (Self(Str).(Str) => NoneType)
* (Self(Str).() => Str) < (Self(Object).() => Object)

的意思。

前者可能看起來很奇怪。雖然是，但將其作為自變量的函數的包含關係卻發生了逆轉。在類型理論中，這種關係（的類型關係）稱為反變（contravariant），相反，的類型關係稱為共變（covariant）。也就是說，可以說函數型是關於自變量的類型的反變，關於返回值的類型的共變。聽起來很複雜，但正如剛才看到的那樣，如果套用實例來考慮的話，這是一個合理的規則。即便如此，如果還不明白的話，可以考慮如下。

Erg 的設計方針中有“輸入的類型大，輸出的類型小”。這可以從函數的變性說起。從上面的規則來看，輸入型是大的一方整體來說是小的類型。因為通用函數明顯比專用函數稀少。而且輸出型越小整體越小。

結果上面的方針等於說“函數的類型最小化”。

## 過錯變性

Erg 還有一種變性。它是非變性的。這是編入型中等具有的變性。這意味著，關於<gtr=“33”/>的 2 個類型<gtr=“34”/>，即使存在包含關係，也不能在<gtr=“35”/>和<gtr=“36”/>之間進行轉換。這是因為是共享參照。有關詳細信息，請參見<gtr=“38”/>。

## 變性指定的全稱類型

可以指定全稱類型的類型變量的上限和下限。


```erg
|A <: T| K(A)
|B :> T| K(B)
```

類型變量列表中的類型變量。在上面的變性說明中，類型變量<gtr=“39”/>是類型<gtr=“40”/>的任何子類，類型變量<gtr=“41”/>是類型<gtr=“42”/>的任何超類。此時，<gtr=“43”/>也稱為<gtr=“44”/>的上限型，<gtr=“45”/>的下限型。

還可以疊加退化規範。


```erg
# U < A < T
{... | A <: T; A :> U}
```

下面是使用變性規範的代碼示例。


```erg
show|S <: Show| s: S = log s

Nil T = Class(Impl=Phantom T)
Cons T = Class(Nil T or List T)
List T = Class {head = T; rest = Cons T}
List(T).
    push|U <: T|(self, x: U): List T = Self.new {head = x; rest = self}
    upcast(self, U :> T): List U = self
```

## 變性指定

請注意中的示例，我們將更詳細地討論這些示例。為了了解上面的代碼，我們需要了解多相型的變性。關於變性，我們在<gtr=“48”/>中進行了詳細說明，但目前需要的事實有以下三個：

* 通常的多相型，等對於<gtr=“50”/>共變（<gtr=“51”/>時<gtr=“52”/>）
* 函數與自變量類型<gtr=“54”/>相反（當<gtr=“55”/>時<gtr=“56”/>）
* 函數與返回類型<gtr=“58”/>共變（當<gtr=“59”/>時<gtr=“60”/>）

例如，可以上播到<gtr=“62”/>，<gtr=“63”/>可以上播到<gtr=“64”/>。

現在，我們將考慮如果省略方法的退化規範會發生什麼情況。


```erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    # List T can be pushed U if T > U
    push|U|(self, x: U): List T = Self.new {head = x; rest = self}
    # List T can be List U if T < U
    upcast(self, U): List U = self
```

即使在這種情況下，Erg 編譯器也可以很好地推論的上限和下限類型。但是，請注意，Erg 編譯器並不理解方法的含義。編譯器只是根據變量和類型變量的使用方式機械地推理和推導類型關係。

如註釋所示，的<gtr=“67”/>類型<gtr=“68”/>是<gtr=“69”/>的子類（如果<gtr=“70”/>，則<gtr=“71”/>等）。即推論為<gtr=“72”/>。此約束禁止更改<gtr=“73”/>參數類型的上傳<gtr=“74”/>（e.g.<gtr=“75”/>）。但是，請注意，<gtr=“76”/>約束並沒有改變函數類型的包含關係。 <gtr=“77”/>這一事實保持不變，只是不能在<gtr=“78”/>方法中執行這樣的上播。同樣，從<gtr=“79”/>到<gtr=“80”/>的轉換在<gtr=“81”/>的約束條件下是可能的，因此可以這樣推論退化規範。此約束禁止更改<gtr=“82”/>的返回類型的上傳<gtr=“83”/>（e.g.<gtr=“84”/>）。

現在，我想如果我允許這個上傳會發生什麼情況。讓我們來反轉退化規範。


```erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    push|U :> T|(self, x: U): List T = Self.new {head = x; rest = self}
    upcast(self, U :> T): List U = self
# TypeWarning: `U` in the `.push` cannot take anything other than `U == T`. Replace `U` with `T`. Or you may have the wrong variance specification.
# TypeWarning: `U` in the `.upcast` cannot take anything other than `U == T`. Replace `U` with `T`. Or you may have the wrong variance specification.
```

只有當同時滿足<gtr=“85”/>約束和<gtr=“86”/>退化規範時，才能滿足。因此，此指定幾乎沒有任何意義。實際上，只允許“上播，如<gtr=“88”/>”=“上播，不改變<gtr=“89”/>”。

## Appendix：用戶定義的變體

用戶定義類型的變性默認為非變。但是，也可以用這一標記軌跡指定變性。如果指定<gtr=“91”/>，則該類型對於<gtr=“92”/>是反變的。如果指定<gtr=“93”/>，則該類型對於<gtr=“94”/>為協變。


```erg
K T = Class(...)
assert not K(Str) <= K(Object)
assert not K(Str) >= K(Object)

InputStream T = Class ..., Impl := Inputs(T)
# Objectを受け入れるストリームは、Strを受け入れるともみなせる
assert InputStream(Str) > InputStream(Object)

OutputStream T = Class ..., Impl := Outputs(T)
# Strを出力するストリームは、Objectを出力するともみなせる
assert OutputStream(Str) < OutputStream(Object)
```