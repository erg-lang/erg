# 程序

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/08_procedure.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/08_procedure.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

处理可变对象时需要过程，但将可变对象作为参数并不一定使其成为过程
这是一个函数接受一个可变对象(不是过程)

```python
peek_str s: Str! = log s
```

<p align='center'>
    <a href='./07_side_effect.md'>上一页</a> | <a href='./09_builtin_procs.md'>下一页</a>
</p>
