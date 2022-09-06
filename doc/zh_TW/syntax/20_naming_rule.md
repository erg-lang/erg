# 命名約定

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/20_naming_rule.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/20_naming_rule.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

如果要將變量用作常量表達式，請確保它以大寫字母開頭。 兩個或多個字母可能是小寫的。

```python
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

具有副作用的對象總是以 `!` 結尾。 程序和程序方法，以及可變類型。
然而，`Proc` 類型本身是不可變的。

```python
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" # `: Proc` 可以省略，因為它是不言自明的
    f -> log "func"
```

如果您想向外界公開一個屬性，請在開頭使用 `.` 定義它。 如果你不把`.`放在開頭，它將是私有的。 為避免混淆，它們不能在同一范圍內共存。

```python
o = {x = 1; .x = 2} # 語法錯誤：同名的私有變量和公共變量不能共存
```

## 文字標識符

可以通過將字符串括在單引號 ('') 中來規避上述規則。 也就是說，程序對象也可以在沒有 `!` 的情況下分配。 但是，在這種情況下，即使該值是常量表達式，也不會被視為常量。
像這樣用單引號括起來的字符串稱為文字標識符。
這在調用Python等其他語言的API(FFI)時使用。

```python
bar! = pyimport("foo").'bar'
```

在 Erg 中也有效的標識符不需要用 '' 括起來。

此外，文字標識符可以包含符號和空格，因此通常不能用作標識符的字符串可以用作標識符。

```python
'?/?t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./19_visibility.md'>上一頁</a> | <a href='./21_lambda.md'>下一頁</a>
</p>