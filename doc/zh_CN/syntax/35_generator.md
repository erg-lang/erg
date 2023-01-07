# 生成器

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_generator.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_generator.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

生成器是在块中使用 `yield!` 过程的特殊过程

```python
g!() =
    yield! 1
    yield! 2
    yield! 3
```

`yield!` 是在调用`self!.yield!` 的子程序块中定义的过程。和`return`一样，它把传递给它的值作为返回值返回，但它具有保存block当前执行状态，再次调用时从头开始执行的特性
生成器既是过程又是迭代器； Python 生成器是一个创建迭代器的函数，而 Erg 直接迭代。过程本身通常不是可变对象(没有`！`)，但生成器是可变对象，因为它自己的内容可以随着每次执行而改变

```python
# Generator!
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

Python 风格的生成器可以定义如下

```python
make_g() = () =>
    yield! 1
    yield! 2
    yield! 3
make_g: () => Generator!
```

<p align='center'>
    <a href='./34_package_system.md'>上一页</a> | Next
</p>
