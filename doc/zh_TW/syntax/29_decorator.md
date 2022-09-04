# 裝飾器（修飾符）

裝飾器用於將特定的狀態和行為添加到類型和函數中，或將其顯式。裝飾師的語法如下。


```erg
@deco
X = ...
```

裝飾器可以有多個，除非衝突。

裝飾器不是一個特殊的對象，它的實體只是一個參數函數。裝飾器等效於以下偽代碼。


```erg
X = ...
X = deco(X)
```

因為 Erg 不能重新賦值變量，所以上面的代碼不能通過。對於簡單的變量，這與相同，但對於即時塊和子程序，這是不可能的，因此需要一個裝飾器。


```erg
@deco
f x =
    y = ...
    x + y

# コードが橫長になるのを防ぐこともできる
@LongNameDeco1
@LongNameDeco2
C = Class ...
```

下面介紹一些頻出的嵌入式裝飾器。

## Inheritable

指示所定義的類型是可繼承類。如果將參數指定為<gtr=“10”/>，則外部模塊類可以繼承這些參數。默認為<gtr=“11”/>，不能從外部繼承。

## Final

使方法不可覆蓋。將類附加到類後，它將成為不可繼承類，但這沒有意義，因為這是缺省類。

## Override

用於覆蓋屬性。缺省情況下，Erg 會在嘗試定義與基類相同的屬性時出錯。

## Impl

指示要實現自變量的特寫。


```erg
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

## Attach

指定默認情況下隨托盤一起提供的附件曲面片。這樣，你就可以重現與 Rust 的trait相同的行為。


```erg
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

這將在從其他模塊導入托盤時自動應用附件修補程序。


```erg
# 本來IntIsBinAdd, OddIsBinAddも同時にインポートする必要があるが、アタッチメントパッチなら省略可
{BinAdd; ...} = import "foo"

assert Int.AddO == Int
assert Odd.AddO == Even
```

在內部，我們只是使用trait的方法將其連接起來。如果發生衝突，可以使用trait的<gtr=“13”/>方法將其移除。


```erg
@Attach X
T = Trait ...
assert X in T.attaches
U = T.detach(X).attach(Y)
assert X not in U.attaches
assert Y in U.attaches
```

## Deprecated

表示變量規範已過時。

## Test

指示測試子程序。測試子例程使用命令執行。

<p align='center'>
    <a href='./28_spread_syntax.md'>Previous</a> | <a href='./30_error_handling.md'>Next</a>
</p>