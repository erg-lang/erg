# 生成器

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_generator.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_generator.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

生成器是在塊中使用 `yield!` 過程的特殊過程

```python
g!() =
    yield! 1
    yield! 2
    yield! 3
```

`yield!` 是在調用`self!.yield!` 的子程序塊中定義的過程。 和`return`一樣，它把傳遞給它的值作為返回值返回，但它具有保存block當前執行狀態，再次調用時從頭開始執行的特性
生成器既是過程又是迭代器； Python 生成器是一個創建迭代器的函數，而 Erg 直接迭代。 過程本身通常不是可變對象(沒有`！`)，但生成器是可變對象，因為它自己的內容可以隨著每次執行而改變

```python
# Generator!
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

Python 風格的生成器可以定義如下

```python
make_g() = () =>
    yield! 1
    yield! 2
    yield! 3
make_g: () => Generator!
```

<p align='center'>
    <a href='./33_package_system.md'>上一頁</a> | Next
</p>
