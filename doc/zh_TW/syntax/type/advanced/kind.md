# kind（Kind）

Erg 中全部都是定型的。套路本身也不例外。表示“類型的類型”的是。例如，<gtr=“14”/>屬於<gtr=“15”/>，<gtr=“16”/>屬於<gtr=“17”/>。 <gtr=“18”/>是最簡單的kind<gtr=“22”/>。在型理論性的記法中，<gtr=“19”/>與<gtr=“20”/>相對應。

在kind這個概念中，實用上重要的是一項以上的kind（多項kind）。 1 項的kind度，例如等屬於它。 1 項kind度表示為<gtr=“31”/>。 <gtr=“25”/>和<gtr=“26”/>等的<gtr=“32”/>是特別將類型作為自變量的多項關係。正如<gtr=“27”/>這一表記所示，實際上<gtr=“28”/>是接受<gtr=“29”/>這一類型並返回<gtr=“30”/>這一類型的函數。但是，由於該函數不是通常意義上的函數，因此通常被稱為 1 項kind（unary kind）。

另外，作為無名函數運算符的本身也可以看作是接收類型並返回類型時的關係。

另外，請注意，非原子kind的kind不是套路。就像是數值但<gtr=“35”/>不是數值一樣，<gtr=“36”/>是類型但<gtr=“37”/>不是類型。等有時也被稱為類型構建子。


```erg
assert not Option in Type
assert Option in Type -> Type
```

因此，下面的代碼會出現錯誤。在 Erg 中可以定義方法的只有原子kind度，方法的第一自變量以外的地方不能使用這個名字。


```erg
# K is an unary kind
K: Type -> Type
K T = Class ...
K.
    foo x = ... # OK，這就像是所謂的靜態方法
    bar self, x = ... # TypeError: cannot define a method to a non-type object
K(T).
    baz self, x = ... # OK
```

二進製或更高類型的示例是 `{T: U}`(: `(Type, Type) -> Type`), `(T, U, V)`(: `(Type, Type, Type) - > Type `), ... 等等。

還有一個零項類型`() -> Type`。這有時等同於類型論中的原子類型，但在 Erg 中有所區別。一個例子是“類”。


```erg
Nil = Class()
```

## kind包含關係

多項關係之間也有部分型關係，原來部分關係。


```erg
K T = ...
L = Inherit K
L <: K
```

也就是說，對於任何，<gtr=“47”/>都是<gtr=“48”/>，反之亦然。


```erg
∀T. L T <: K T <=> L <: K
```

## 高階kind

還有一種叫做高階kind（higher-order kind）。這是與高階函數相同概念的kind，是接受kind本身的kind。等是高階kind度。嘗試定義屬於高階kind度的對象。


```erg
IntContainerOf K: Type -> Type = K Int
assert IntContainerOf Option == Option Int
assert IntContainerOf Result == Result Int
assert IntContainerOf in (Type -> Type) -> Type
```

多項kind度的約束變量通常表示為 K，L，...等（K 是 Kind 的 K）。

## 套管

在型理論中，有一個叫做記錄的概念。這與 Erg 的記錄基本相同。


```erg
# This is a record, and it corresponds to what is called a record in type theory
{x = 1; y = 2}
```

當記錄的值全部為類型時，它被稱為記錄類型，是類型的一種。


```erg
assert {x = 1; y = 2} in {x = Int; y = Int}
```

記錄類型用於輸入記錄。善於體諒的人可能會認為，應該有“唱片kind”來定型唱片型。實際上，它是存在的。


```erg
log Typeof {x = Int; y = Int} # {{x = Int; y = Int}}
```

像這樣的類型就是唱片kind。這不是特別的記法。它是只以為要素的枚舉型。


```erg
Point = {x = Int; y = Int}
Pointy = {Point}
```

記錄kind的重要特性在於，當<gtr=“54”/>時，<gtr=“55”/>。這從列舉型實際上是篩子型的糖衣句法就可以看出。


```erg
# 通常のオブジェクトでは{c} == {X: T | X == c}だが、
# 型の場合等號が定義されない場合があるので|T| == {X | X <: T}となる
{Point} == {P | P <: Point}
```

型限制中的實際上是的糖衣句法。這種類型的組套即kind一般被稱為組套kind。設定kind也出現在 Iterator 模式中。


```erg
Iterable T = Trait {
    .Iterator = {Iterator}
    .iter = Self(T).() -> Self.Iterator T
}
```

## 多項關係型推理


```erg
Container K: Type -> Type, T: Type = Patch K(T, T)
Container(K).
    f self = ...
Option T: Type = Patch T or NoneType
Option(T).
    f self = ...
Fn T: Type = Patch T -> T
Fn(T).
    f self = ...
Fn2 T, U: Type = Patch T -> U
Fn2(T, U).
    f self = ...

(Int -> Int).f() # どれが選択される?
```

在上面的例子中，方法選擇哪個補丁呢？簡單地說，<gtr=“61”/>被認為是可以選擇的，但<gtr=“62”/>也有可能，<gtr=“63”/>包含<gtr=“64”/>的原樣，因此任意類型都適用，<gtr=“65”/>也是<gtr=“58”/>，即<gtr=“59”/>與<gtr=“66”/>相匹配。因此，上面的 4 個補丁都可以作為選擇。

在這種情況下，根據以下優先標準選擇補丁。

* 任何（e.g.<gtr=“68”/>）比<gtr=“69”/>優先匹配<gtr=“70”/>。
* 任何（e.g.<gtr=“72”/>）比<gtr=“73”/>優先匹配<gtr=“74”/>。
* 同樣的標準適用於 3 項以上的kind度。
* 選擇替換類型變量較少的變量。例如，優先匹配<gtr=“78”/>（替換類型變量：T），而不是<gtr=“76”/>（替換類型變量：K，T）或<gtr=“77”/>（替換類型變量：T，U）。
* 如果替換數相同，則錯誤為“無法選擇”。

---

<span id="1" style="font-size:x-small">在1<gtr=“82”/>型理論的記法中<gtr=“79”/><gtr=“80”/></span>

<span id="2" style="font-size:x-small">2<gtr=“85”/>存在可視性等微妙的差異。 <gtr=“83”/></span>