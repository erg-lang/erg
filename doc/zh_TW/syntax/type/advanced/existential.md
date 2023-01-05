# 存在類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/existential.md%26commit_hash%3D44d7784aac3550ba97c8a1eaf20b9264b13d4134)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/existential.md&commit_hash=44d7784aac3550ba97c8a1eaf20b9264b13d4134)

如果存在對應于?的for-all類型，那么很自然地假設存在對應于?的存在類型
存在類型并不難。你已經知道存在類型，只是沒有意識到它本身

```python
T: Trait
f x: T = ...
```

上面的 trait `T` 被用作存在類型
相比之下，小寫的`T`只是一個Trait，`X`是一個for-all類型

```python
f|X <: T| x: X = ...
```

事實上，existential 類型被 for-all 類型所取代。那么為什么會有存在類型這樣的東西呢?
首先，正如我們在上面看到的，存在類型不涉及類型變量，這簡化了類型規范
此外，由于可以刪除類型變量，因此如果它是一個全推定類型，則可以構造一個等級為 2 或更高的類型

```python
show_map f: (|T| T -> T), arr: [Show; _] =
    arr.map x ->
        y = f x
        log y
        y
```

但是，如您所見，existential 類型忘記或擴展了原始類型，因此如果您不想擴展返回類型，則必須使用 for-all 類型
相反，僅作為參數且與返回值無關的類型可以寫為存在類型

```python
# id(1): 我希望它是 Int
id|T|(x: T): T = x
# |S <: Show|(s: S) -> () 是多余的
show(s: Show): () = log s
```

順便說一句，類不稱為存在類型。一個類不被稱為存在類型，因為它的元素對象是預定義的
存在類型是指滿足某種Trait的任何類型，它不是知道實際分配了什么類型的地方。