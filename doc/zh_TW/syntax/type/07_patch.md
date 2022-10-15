# 修補

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/07_patch.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/07_patch.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

Erg 不允許修改現有類型和類
這意味著，不可能在類中定義額外的方法，也不能執行特化(一種語言特性，單態化多態聲明的類型并定義專用方法，如在 C++ 中)
但是，在許多情況下，您可能希望向現有類型或類添加功能，并且有一個稱為"修補"的功能允許您執行此操作

```python
StrReverse = Patch Str
StrReverse.
    reverse self = self.iter().rev().collect(Str)

assert "abc".reverse() == "cba"
```

補丁的名稱應該是要添加的主要功能的簡單描述
這樣，被修補類型的對象(`Str`)可以使用修補程序的方法(`StrReverse`)
實際上，內置方法`.reverse`并不是`Str`的方法，而是`StrRReverse`中添加的方法

但是，補丁方法的優先級低于名義類型(類/trait)的方法，并且不能覆蓋現有類型

```python
StrangeInt = Patch Int
StrangeInt.
    `_+_` = Int.`_-_` # 賦值錯誤: . `_+_` 已在 Int 中定義
```

如果要覆蓋，則必須從類繼承
但是，基本上建議不要覆蓋并定義具有不同名稱的方法
由于一些安全限制，覆蓋不是很容易做到

```python
StrangeInt = Inherit Int
StrangeInt.
# 覆蓋方法必須被賦予覆蓋裝飾器
    # 另外，你需要覆蓋所有依賴于 Int.`_+_` 的 Int 方法
    @Override
    `_+_` = Super.`_-_` # OverrideError: Int.`_+_` 被 ... ````` 引用，所以這些方法也必須被覆蓋
```

## 選擇修補程序

可以為單一類型定義修復程序，并且可以組合在一起

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
StrMultiReplace = Patch(Str)
StrMultiReverse.
    multi_replace self, pattern_and_targets: [(Pattern, Str)] = ...
StrToCamelCase = Patch(Str)
StrToCamelCase.
    to_camel_case self = ...
StrToKebabCase = Patch(Str)
StrToKebabCase.
    to_kebab_case self = ...

StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
```

```python
{StrBoosterPack; ...} = import "foo"

assert "abc".reverse() == "cba"
assert "abc".multi_replace([("a", "A"), ("b", "B")]) == "ABc"
assert "to camel case".to_camel_case() == "toCamelCase"
assert "to kebab case".to_kebab_case() == "to-kebab-case"
```

如果定義了多個修復程序，其中一些可能會導致重復實施

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
# 更高效的實現
StrReverseMk2 = Patch(Str)
StrReverseMk2.
    reverse self = ...

"hello".reverse() # 補丁選擇錯誤: `.reverse` 的多個選擇: StrReverse, StrReverseMk2
```

在這種情況下，您可以使用 __related function__ 形式而不是方法形式使其唯一

```python
assert StrReverseMk2.reverse("hello") == "olleh"
```

You can also make it unique by selectively importing.

```python
{StrReverseMk2; ...} = import "foo"

assert "hello".reverse() == "olleh"
```

## 膠水補丁

維修程序也可以將類型相互關聯。`StrReverse` 補丁涉及 `Str` 和 `Reverse`
這樣的補丁稱為 __glue patch__
因為 `Str` 是內置類型，所以用戶需要使用膠水補丁來改造Trait

```python
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch Str, Impl := Reverse
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

每個類型/Trait對只能定義一個膠水補丁
這是因為如果多個膠水修復程序同時"可見"，就不可能唯一確定選擇哪個實現
但是，當移動到另一個范圍(模塊)時，您可以交換維修程序

```python
NumericStr = Inherit Str
NumericStr.
    ...

NumStrRev = Patch NumericStr, Impl := Reverse
NumStrRev.
    ...
# 重復修補程序錯誤: 數值Str已與"反向"關聯`
# 提示: 'Str'(NumericStr'的超類)通過'StrReverse'與'Reverse'關聯
```

## 附錄: 與 Rust Trait的關系

Erg 修復程序相當于 Rust 的(改造的)`impl` 塊

```rust
// Rust
trait Reverse {
    fn reverse(self) -> Self;
}

impl Reverse for String {
    fn reverse(self) -> Self {
        self.chars().rev().collect()
    }
}
```

可以說，Rust 的Trait是 Erg 的Trait和修復程序的Trait。這使得 Rust 的Trait聽起來更方便，但事實并非如此

```python
# Erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch(Str, Impl := Reverse)
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

因為 impl 塊在 Erg 中被對象化為補丁，所以在從其他模塊導入時可以選擇性地包含。作為副作用，它還允許將外部Trait實現到外部結構
此外，結構類型不再需要諸如 `dyn trait` 和 `impl trait` 之類的語法

```python
# Erg
reversible: [Reverse; 2] = [[1, 2, 3], "hello"]

iter|T|(i: Iterable T): Iterator T = i.iter()
```

```rust
// Rust
let reversible: [Box<dyn Reverse>; 2] = [Box::new([1, 2, 3]), Box::new("hello")];

fn iter<I>(i: I) -> impl Iterator<Item = I::Item> where I: IntoIterator {
    i.into_iter()
}
```

## 通用補丁

補丁不僅可以為一種特定類型定義，還可以為"一般功能類型"等定義
在這種情況下，要給出自由度的項作為參數給出(在下面的情況下，`T: Type`)。以這種方式定義的補丁稱為全對稱補丁
如您所見，全對稱補丁正是一個返回補丁的函數，但它本身也可以被視為補丁

```python
FnType T: Type = Patch(T -> T)
FnType(T).
    type = T

assert (Int -> Int).type == Int
```

## 結構補丁

此外，可以為滿足特定結構的任何類型定義修復程序
但是，這比名義上的維修程序和類方法具有較低的優先級

在定義結構修復程序時應使用仔細的設計，因為某些屬性會因擴展而丟失，例如以下內容

```python
# 這不應該是 `Structural`
Norm = Structural Patch {x = Int; y = Int}
Norm.
    norm self = self::x**2 + self::y**2

Point2D = Class {x = Int; y = Int}
assert Point2D.new({x = 1; y = 2}).norm() == 5

Point3D = Class {x = Int; y = Int; z = Int}
assert Point3D.new({x = 1; y = 2; z = 3}).norm() == 14 # AssertionError:
```

<p align='center'>
    <a href='./06_nst_vs_sst.md'>上一頁</a> | <a href='./08_value.md'>下一頁</a>
</p>
