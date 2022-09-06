# 裝飾器(修飾符)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_decorator.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_decorator.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

裝飾器用于向類型或函數添加或演示特定狀態或行為。
裝飾器的語法如下。

```python
@deco
X=...
```

你可以有多個裝飾器，只要它們不沖突。

裝飾器不是一個特殊的對象，它只是一個單參數函數。 裝飾器等價于下面的偽代碼。

```python
X=...
X = deco(X)
```

Erg 不允許重新分配變量，因此上面的代碼不起作用。
對于簡單的變量，它與`X = deco(...)` 相同，但對于即時塊和子例程，你不能這樣做，所以你需要一個裝飾器。

```python
@deco
f x =
    y = ...
    x + y

# 還可以防止代碼變成水平的
@LongNameDeco1
@LongNameDeco2
C = Class...
```

下面是一些常用的內置裝飾器。

## 可繼承

指示定義類型是可繼承的類。 如果為參數 `scope` 指定 `"public"`，甚至可以繼承外部模塊的類。 默認情況下它是`"private"`，不能被外部繼承。

＃＃ 最后

使該方法不可覆蓋。 將它添加到類中使其成為不可繼承的類，但由于它是默認值，因此沒有意義。

## 覆蓋

覆蓋屬性時使用。 默認情況下，如果您嘗試定義與基類相同的屬性，Erg 將拋出錯誤。

## 實現

表示參數 trait 已實現。

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
Sub = Trait {
    .`_-_` = Self.(Self) -> Self
}

C = Class({i = Int}, Impl := Add and Sub)
C.
    @Impl Add
    `_+_` self, other = C.new {i = self::i + other::i}
    @Impl Sub
    `_-_` self, other = C.new {i = self::i - other::}
```

## 附

指定默認情況下隨 trait 附帶的附件補丁。
這允許您重現與 Rust 特征相同的行為。

```python
# foo.er
Add R = Trait {
    .AddO = Type
    .`_+_` = Self.(R) -> Self.AddO
}
@Attach AddForInt, AddForOdd
ClosedAdd = Subsume Add(Self)

AddForInt = Patch(Int, Impl := ClosedAdd)
AddForInt.AddO = Int
AddForOdd = Patch(Odd, Impl := ClosedAdd)
AddForOdd.AddO = Even
```

當從其他模塊導入特征時，這將自動應用附件補丁。

```Python
# 本來應該同時導入IntIsBinAdd和OddIsBinAdd，但是如果是附件補丁可以省略
{BinAdd; ...} = import "foo"

assert Int. AddO == Int
assert Odd.AddO == Even
```

在內部，它只是使用 trait 的 .attach 方法附加的。 可以使用 trait 的 `.detach` 方法消除沖突。

```python
@Attach X
T = Trait...
assert X in T. attaches
U = T.detach(X).attach(Y)
assert X not in U. attaches
assert Y in U. attaches
```

## 已棄用

指示變量規范已過時且不推薦使用。

＃＃ 測試

表示這是一個測試子例程。 測試子程序使用 `erg test` 命令運行。

<p align='center'>
    <a href='./28_spread_syntax.md'>上一頁</a> | <a href='./30_error_handling.md'>下一頁</a>
</p>