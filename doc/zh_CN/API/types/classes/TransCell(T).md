# TransCell! T: Type!

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/TransCell(T).md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/TransCell(T).md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

它是一个单元格，其内容可以针对每个模具进行更改。 由于它是T类型的子类型，因此它也表现为T类型
当它在初始化时输入T时很有用，并且在某个点之后总是输入U

```python
a = TransCell!.new None
a: TransCell! !NoneType
a.set! 1
a: TransCell! !Int
assert a + 1 == 2
```
