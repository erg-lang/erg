# 類型推斷算法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/inference.md%26commit_hash%3D00350f64a40b12f763a605bc16748d09379ab182)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/inference.md&commit_hash=00350f64a40b12f763a605bc16748d09379ab182)

> __Warning__: 此部分正在編輯中，可能包含一些錯誤

顯示了下面使用的符號

```python
Free type variables (type, unbound): ?T, ?U, ...
Free-type variables (values, unbound): ?a, ?b, ...
type environment (Γ): { x: T, ... }
Type assignment rule (S): { ?T --> T, ... }
Type argument evaluation environment (E): { e -> e', ... }
```

我們以下面的代碼為例: 

```python
v = ![]
v.push! 1
print! v
```

Erg 的類型推斷主要使用 Hindley-Milner 類型推斷算法(盡管已經進行了各種擴展)。具體而言，類型推斷是通過以下過程執行的。術語將在后面解釋

1. 推斷右值的類型(搜索)
2. 實例化結果類型
3. 如果是調用，執行類型替換(substitute)
4. 解決已經單態化的特征
5. 如果有類型變量值，求值/歸約(eval)
6. 刪除鏈接類型變量(deref)
7. 傳播可變依賴方法的變化
8. 如果有左值并且是Callable，則泛化參數類型(generalize)
9. 如果有左值，對(返回值)類型進行泛化(generalize)
10. 如果是賦值，則在符號表(`Context`)中注冊類型信息(更新)

具體操作如下

第 1 行。Def{sig: v, block: ![]}
    獲取塊類型: 
        獲取 UnaryOp 類型: 
            getArray 類型: `['T; 0]`
            實例化: `[?T; 0]`
            (替代，評估被省略)
    更新: `Γ: {v: [?T; 0]！}`
    表達式 返回`NoneType`: OK

第 2 行 CallMethod {obj: v, name: push!, args: [1]}
    獲取 obj 類型: `Array!(?T, 0)`
        搜索: `Γ Array!(?T, 0).push!({1})`
        得到: `= Array!('T ~> 'T, 'N ~> 'N+1).push!('T) => NoneType`
        實例化: `Array!(?T, ?N).push!(?T) => NoneType`
        替代(`S: {?T --> Nat, ?N --> 0}`): `Array!(Nat ~> Nat, 0 ~> 0+1).push!(Nat) => NoneType`
        評估: `Array!(Nat, 0 ~> 1).push!({1}) => NoneType`
        更新: `Γ: {v: [Nat; 1]！}`
    表達式 返回`NoneType`: OK

第 3 行。調用 {obj: print!, args: [v]}
    獲取參數類型: `[[Nat; 1]!]`
    獲取 obj 類型: 
        搜索: `Γ print!([Nat; 1]!)`
        得到: `= print!(...Object) => NoneType`
    表達式 返回`NoneType`: OK

## 類型變量的實現

類型變量最初在 [ty.rs] 的 `Type` 中表示如下。它現在以不同的方式實現，但本質上是相同的想法，所以我將以更天真的方式考慮這種實現
`RcCell<T>` 是 `Rc<RefCell<T>>` 的包裝類型

```rust
pub enum Type {
    ...
    Var(RcCell<Option<Type>>), // a reference to the type of other expression, see docs/compiler/inference.md
    ...
}
```

類型變量可以通過將實體類型保存在外部字典中來實現，并且類型變量本身只有它的鍵。但是，據說使用 `RcCell` 的實現通常更有效(需要驗證，[來源](https://mobile.twitter.com/bd_gfngfn/status/1296719625086877696?s=21))

類型變量首先被初始化為 `Type::Var(RcCell::new(None))`
當分析代碼并確定類型時，會重寫此類型變量
如果內容直到最后都保持為 None ，它將是一個無法確定為具體類型的類型變量(當場)。例如，具有 `id x = x` 的 `x` 類型
我將這種狀態下的類型變量稱為 __Unbound 類型變量__(我不知道確切的術語)。另一方面，我們將分配了某種具體類型的變量稱為 __Linked 類型變量__

兩者都是自由類型變量(該術語顯然以"自由變量"命名)。這些是編譯器用于推理的類型變量。它之所以有這樣一個特殊的名字，是因為它不同于程序員指定類型的類型變量，例如 `id: 'T -> 'T` 中的 `'T`

未綁定類型變量表示為`?T`、`?U`。在類型論的上下文中，經常使用 α 和 β，但這一種是用來簡化輸入的
請注意，這是出于一般討論目的而采用的表示法，實際上并未使用字符串標識符實現

進入類型環境時，未綁定的類型變量 `Type::Var` 被替換為 `Type::MonoQuantVar`。這稱為 __quantified 類型變量__。這類似于程序員指定的類型變量，例如"T"。內容只是一個字符串，并沒有像自由類型變量那樣鏈接到具體類型的工具

用量化類型變量替換未綁定類型變量的操作稱為__generalization__(或泛化)。如果將其保留為未綁定類型變量，則類型將通過一次調用固定(例如，調用 `id True` 后，`id 1` 的返回類型將是 `Bool`)，所以它必須是概括的
以這種方式，在類型環境中注冊了包含量化類型變量的通用定義

## 概括、類型方案、具體化

讓我們將未綁定類型變量 `?T` 泛化為 `gen` 的操作表示。令生成的廣義類型變量為 `|T: Type| T`
在類型論中，量化類型，例如多相關類型 `α->α`，通過在它們前面加上 `?α.` 來區分(像 ? 這樣的符號稱為(通用)量詞。)
這樣的表示(例如`?α.α->α`)稱為類型方案。Erg 中的類型方案表示為 `|T: Type| T -> T`
類型方案通常不被認為是一流的類型。以這種方式配置類型系統可以防止類型推斷起作用。但是，在Erg中，在一定條件下可以算是一流的類型。有關詳細信息，請參閱 [rank2 類型](../syntax/type/advanced/_rank2type.md)

現在，當在使用它的類型推斷(例如，`id 1`，`id True`)中使用獲得的類型方案(例如`'T -> 'T(id's type scheme)`)時，必須釋放generalize。這種逆變換稱為 __instantiation__。我們將調用操作`inst`

```python
gen ?T = 'T
inst 'T = ?T (?T ? Γ)
```

重要的是，這兩個操作都替換了所有出現的類型變量。例如，如果你實例化 `'T -> 'T`，你會得到 `?T -> ?T`
實例化需要替換 dict，但為了泛化，只需將 `?T` 與 `'T` 鏈接以替換它

之后，給出參數的類型以獲取目標類型。此操作稱為類型替換，將用 `subst` 表示
此外，如果表達式是調用，則獲取返回類型的操作表示為 `subst_call_ret`。第一個參數是參數類型列表，第二個參數是要分配的類型

類型替換規則 `{?T --> X}` 意味著將 `?T` 和 `X` 重寫為相同類型。此操作稱為 __Unification__。`X` 也可以是類型變量
[單獨部分] 中描述了詳細的統一算法。我們將統一操作表示為"統一"

```python
unify(?T, Int) == Ok(()) # ?T == (Int)

# S為類型分配規則，T為適用類型
subst(S: {?T --> X}, T: ?T -> ?T) == X -> X
# Type assignment rules are {?T --> X, ?U --> T}
subst_call_ret([X, Y], (?T, ?U) -> ?U) == Y
```

## 半統一(semi-unification)

統一的一種變體稱為半統一(__Semi-unification__)。這是更新類型變量約束以滿足子類型關系的操作
在某些情況下，類型變量可能是統一的，也可能不是統一的，因此稱為"半"統一

例如，在參數分配期間會發生半統一
因為實際參數的類型必須是形式參數類型的子類型
如果參數類型是類型變量，我們需要更新子類型關系以滿足它

```python
# 如果形參類型是T
f(x: T): T = ...

a: U
# 必須為 U <: T，否則類型錯誤
f(a)
```

## 泛化

泛化不是一項簡單的任務。當涉及多個作用域時，類型變量的"級別管理"就變得很有必要了
為了看到層級管理的必要性，我們首先確認沒有層級管理的類型推斷會導致問題
推斷以下匿名函數的類型

```python
x ->
    y = x
    y
```

首先，Erg 分配類型變量如下: 
y 的類型也是未知的，但暫時未分配

```python
x(: ?T) ->
    y = x
    y
```

首先要確定的是右值 x 的類型。右值是一種"用途"，因此我們將其具體化
但是 x 的類型 `?T` 已經被實例化了，因為它是一個自由變量。Yo`?T` 成為右值的類型

```python
x(: ?T) ->
    y = x (: inst ?T)
    y
```

注冊為左值 y 的類型時進行泛化。然而，正如我們稍后將看到的，這種概括是不完善的，并且會產生錯誤的結果

```python
x(: ?T) ->
    y(:gen?T) = x(:?T)
    y
```

```python
x(: ?T) ->
    y(: 'T) = x
    y
```

y 的類型現在是一個量化類型變量"T"。在下一行中，立即使用 `y`。具體的

```python
x: ?T ->
    y(: 'T) = x
    y(: inst 'T)
```

請注意，實例化必須創建一個與任何已經存在的(自由)類型變量不同的(自由)類型變量(概括類似)。這樣的類型變量稱為新類型變量

```python
x: ?T ->
    y = x
    y(: ?U)
```

并查看生成的整個表達式的類型。`?T -> ?U`
但顯然這個表達式應該是`?T -> ?T`，所以我們知道推理有問題
發生這種情況是因為我們沒有"級別管理"類型變量

所以我們用下面的符號來介紹類型變量的層次。級別表示為自然數

```python
# 普通類型變量
?T<1>, ?T<2>, ...
# 具有子類型約束的類型變量
?T<1>(<:U) or ?T(<:U)<1>, ...
```

讓我們再嘗試一次: 

```python
x ->
    y = x
    y
```

首先，按如下方式分配一個 leveled 類型變量:  toplevel 級別為 1。隨著范圍的加深，級別增加
函數參數屬于內部范圍，因此它們比函數本身高一級

```python
# level 1
x (: ?T<2>) ->
    # level 2
    y = x
    y
```

首先，實例化右值`x`。和以前一樣，沒有任何改變

```python
x (: ?T<2>) ->
    y = x (: inst ?T<2>)
    y
```

這是關鍵。這是分配給左值`y`的類型時的概括
早些時候，這里的結果很奇怪，所以我們將改變泛化算法
如果類型變量的級別小于或等于當前范圍的級別，則泛化使其保持不變

```python
gen ?T<n> = if n <= current_level, then= ?T<n>, else= 'T
```

```python
x (: ?T<2>) ->
    # current_level = 2
    y(: gen ?T<2>) = x(: ?T<2>)
    y
```

That is, the lvalue `y` has type `?T<2>`.

```python
x (: ?T<2>) ->
    # ↓ 不包括
    y(: ?T<2>) = x
    y
```

y 的類型現在是一個未綁定的類型變量 `?T<2>`。具體如下幾行: 但是 `y` 的類型沒有被概括，所以什么也沒有發生

```python
x (: ?T<2>) ->
    y(: ?T<2>) = x
    y (: inst ?T<2>)
```

```python
x (: ?T<2>) ->
    y = x
    y (: ?T<2>)
```

我們成功獲得了正確的類型`?T<2> -> ?T<2>`

讓我們看另一個例子。這是更一般的情況，具有函數/運算符應用程序和前向引用

```python
fx, y = id(x) + y
id x = x

f10,1
```

讓我們逐行瀏覽它

在 `f` 的推斷過程中，會引用后面定義的函數常量 `id`
在這種情況下，在 `f` 之前插入一個假設的 `id` 聲明，并為其分配一個自由類型變量
注意此時類型變量的級別是`current_level`。這是為了避免在其他函數中泛化

```python
id: ?T<1> -> ?U<1>
f x (: ?V<2>), y (: ?W<2>) =
    id(x) (: subst_call_ret([inst ?V<2>], inst ?T<1> -> ?U<1>)) + y
```

類型變量之間的統一將高級類型變量替換為低級類型變量
如果級別相同，則無所謂

類型變量之間的半統一有點不同
不同級別的類型變量不得相互施加類型約束

```python
# BAD
f x (: ?V<2>), y (: ?W<2>) =
    # ?V<2>(<: ?T<1>)
    # ?T<1>(:> ?V<2>)
    id(x) (: ?U<1>) + y (: ?W<2>)
```

這使得無法確定在何處實例化類型變量
對于 Type 類型變量，執行正常統一而不是半統一
也就是說，統一到下層

```python
# OK
f x (: ?V<2>), y (: ?W<2>) =
    # ?V<2> --> ?T<1>
    id(x) (: ?U<1>) + y (: ?W<2>)
```

```python
f x (: ?T<1>), y (: ?W<2>) =
    (id(x) + x): subst_call_ret([inst ?U<1>, inst ?W<2>], inst |'L <: Add('R)| ('L, 'R) -> 'L .AddO)
```

```python
f x (: ?T<1>), y (: ?W<2>) =
    (id(x) + x): subst_call_ret([inst ?U<1>, inst ?W<2>], (?L(<: Add(?R<2>))<2>, ?R<2 >) -> ?L<2>.AddO)
```

```python
id: ?T<1> -> ?U<1>
f x (: ?T<1>), y (: ?W<2>) =
    # ?U<1>(<: Add(?W<2>)) # Inherit the constraints of ?L
    # ?L<2> --> ?U<1>
    # ?R<2> --> ?W<2> (not ?R(:> ?W), ?W(<: ?R))
    (id(x) + x) (: ?U<1>.AddO)
```

```python
# current_level = 1
f(x, y) (: gen ?T<1>, gen ?W<2> -> gen ?U<1>.AddO) =
    id(x) + x
```

```python
id: ?T<1> -> ?U<1>
f(x, y) (: |'W: Type| (?T<1>, 'W) -> gen ?U<1>(<: Add(?W<2>)).AddO) =
    id(x) + x
```

```python
f(x, y) (: |'W: Type| (?T<1>, 'W) -> ?U<1>(<: Add(?W<2>)).AddO) =
    id(x) + x
```

定義時，提高層次，使其可以泛化

```python
# ?T<1 -> 2>
# ?U<1 -> 2>
id x (: ?T<2>) -> ?U<2> = x (: inst ?T<2>)
```

如果已經分配了返回類型，則與結果類型統一(`?U<2> --> ?T<2>`)

```python
# ?U<2> --> ?T<2>
f(x, y) (: |'W: Type| (?T<2>, 'W) -> ?T<2>(<: Add(?W<2>)).AddO) =
    id(x) + x
# current_level = 1
id(x) (: gen ?T<2> -> gen ?T<2>) = x (: ?T<2>)
```

如果類型變量已經被實例化為一個簡單的類型變量，
依賴于它的類型變量也將是一個 Type 類型變量
廣義類型變量對于每個函數都是獨立的

```python
f(x, y) (: |'W: Type, 'T <: Add('W)| ('T, 'W) -> 'T.AddO) =
    id(x) + x
id(x) (: |'T: Type| 'T -> gen 'T) = x
```

```python
f x, y (: |'W: Type, 'T <: Add('W)| ('T, 'W) -> 'T.AddO) =
    id(x) + y
id(x) (: 'T -> 'T) = x

f(10, 1) (: subst_call_ret([inst {10}, inst {1}], inst |'W: Type, 'T <: Add('W)| ('T, 'W) -> 'T .AddO)
```

```python
f(10, 1) (: subst_call_ret([inst {10}, inst {1}], (?T<1>(<: Add(?W<1>)), ?W<1>) -> ? T<1>.AddO))
```

類型變量綁定到具有實現的最小類型

```python
# ?T(:> {10} <: Add(?W<1>))<1>
# ?W(:> {1})<1>
# ?W(:> {1})<1> <: ?T<1> (:> {10}, <: Add(?W(:> {1})<1>))
# serialize
# {1} <: ?W<1> or {10} <: ?T<1> <: Add({1}) <: Add(?W<1>)
# Add(?W)(:> ?V) 的最小實現特征是 Add(Nat) == Nat，因為 Add 相對于第一個參數是協變的
# {10} <: ?W<1> or {1} <: ?T<1> <: Add(?W<1>) <: Add(Nat) == Nat
# ?T(:> ?W(:> {10}) or {1}, <: Nat).AddO == Nat # 如果只有一個候選人，完成評估
f(10, 1) (: (?W(:> {10}, <: Nat), ?W(:> {1})) -> Nat)
# 程序到此結束，所以去掉類型變量
f(10, 1) (: ({10}, {1}) -> Nat)
```

整個程序的結果類型是: 

```python
f|W: Type, T <: Add(W)|(x: T, y: W): T.AddO = id(x) + y
id|T: Type|(x: T): T = x

f(10, 1): Nat
```

我還重印了原始的、未明確鍵入的程序

```python
fx, y = id(x) + y
id x = x

f(10, 1)
```