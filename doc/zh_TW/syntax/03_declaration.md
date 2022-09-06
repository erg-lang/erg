# 宣言(Declaration)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/03_declaration.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/03_declaration.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

聲明是用于指定要使用的變量類型的語法。
可以在代碼中的任何地方進行聲明，但單獨的聲明并不引用變量。 它們必須被初始化。
分配后，可以檢查聲明以確保類型與分配它的對象兼容。

```python
i: Int
# 可以與賦值同時聲明，如 i: Int = 2
i = 2
i: Num
i: Nat
i: -2..2
i: {2}
```

賦值后的聲明類似于`assert`的類型檢查，但具有在編譯時檢查的特點。
在運行時通過`assert`進行類型檢查可以檢查“可能是Foo類型”，但是在編譯時通過`:`進行類型檢查是嚴格的：如果類型未確定為“類型Foo”，則不會通過 檢查會出現錯誤。

```python
i = (-1..10).sample!
assert i in Nat # 這可能會通過
i: Int # 這會通過
i: Nat # 這不會通過(-1 不是 Nat 的元素)
```

函數可以用兩種不同的方式聲明。

```python
f: (x: Int, y: Int) -> Int
f: (Int, Int) -> Int
```

如果顯式聲明參數名稱，如果在定義時名稱不同，則會導致類型錯誤。 如果你想給參數名稱任意命名，你可以用第二種方式聲明它們。 在這種情況下，類型檢查只會看到方法名稱及其類型。

```python
T = Trait {
    .f = (x: Int, y: Int): Int
}

C = Class(U, Impl := T)
C.f(a: Int, b: Int): Int = ... # 類型錯誤：`.f` 必須是 `(x: Int, y: Int) -> Int` 的類型，而不是 `(a: Int, b: Int) -> Int`
```

<p align='center'>
    <a href='./02_name.md'>上一頁</a> | <a href='./04_function.md'>下一頁</a>
</p>
