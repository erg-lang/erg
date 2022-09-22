# 細化類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/12_refinement.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/12_refinement.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

細化類型是受謂詞表達式約束的類型。 枚舉類型和區間類型是細化類型的語法糖。

細化類型的標準形式是`{Elem: Type | (預)*}`。 這意味著該類型是其元素為滿足 `Pred` 的 `Elem` 的類型。
可用于篩選類型的類型僅為 [Const type]。

```python
Nat = 0.. _
Odd = {N: Int | N % 2 == 1}
Char = StrWithLen 1
# StrWithLen 1 == {_: StrWithLen N | N == 1}
[Int; 3] == {_: Array Int, N | N == 3}
Array3OrMore == {A: Array _, N | N >= 3}
```

當有多個 pred 時，可以用 `;` 或 `and` 或 `or` 分隔。 `;` 和 `and` 的意思是一樣的。

`Odd` 的元素是 `1, 3, 5, 7, 9, ...`。
它被稱為細化類型，因為它的元素是現有類型的一部分，就好像它是細化一樣。

`Pred` 被稱為(左側)謂詞表達式。 和賦值表達式一樣，它不返回有意義的值，左側只能放置一個模式。
也就是說，諸如`X**2 - 5X + 6 == 0`之類的表達式不能用作細化類型的謂詞表達式。 在這方面，它不同于右側的謂詞表達式。

```python
{X: Int | X**2 - 5X + 6 == 0} # 語法錯誤：謂詞形式無效。 只有名字可以在左邊
```

如果你知道如何解二次方程，你會期望上面的細化形式等價于`{2, 3}`。
但是，Erg 編譯器對代數的了解很少，因此無法解決右邊的謂詞。

## 智能投射

很高興您定義了 `Odd`，但事實上，它看起來不能在文字之外使用太多。 要將普通 `Int` 對象中的奇數提升為 `Odd`，即將 `Int` 向下轉換為 `Odd`，您需要傳遞 `Odd` 的構造函數。
對于細化類型，普通構造函數 `.new` 可能會出現恐慌，并且有一個名為 `.try_new` 的輔助構造函數返回一個 `Result` 類型。

```python
i = Odd.new (0..10).sample!()
i: Odd # or Panic
```

它也可以用作 `match` 中的類型說明。

```python
# i: 0..10
i = (0..10).sample!
match i:
    o: Odd ->
        log "i: Odd"
    n: Nat -> # 0..10 < Nat
        log "i: Nat"
```

但是，Erg 目前無法做出諸如"偶數"之類的子決策，因為它不是"奇數"等。

## 枚舉、區間和篩選類型

前面介紹的枚舉/區間類型是細化類型的語法糖。
`{a, b, ...}` 是 `{I: Typeof(a) | I == a 或 I == b 或 ... }`，并且 `a..b` 被去糖化為 `{I: Typeof(a) | 我 >= a 和我 <= b}`。

```python
{1, 2} == {I: Int | I == 1 or I == 2}
1..10 == {I: Int | I >= 1 and I <= 10}
1... <10 == {I: Int | I >= 1 and I < 10}
```

## 細化模式

正如 `_: {X}` 可以重寫為 `X`(常量模式)，`_: {X: T | Pred}` 可以重寫為`X: T | Pred`

```python
# 方法 `.m` 是為長度為 3 或更大的數組定義的
Array(T, N | N >= 3)
    .m(&self) = ...
```
