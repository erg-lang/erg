# TransCell! T: Type!

它是一个单元格，其内容可以针对每个模具进行更改。 由于它是T类型的子类型，因此它也表现为T类型
当它在初始化时输入T时很有用，并且在某个点之后总是输入U

```python
a = TransCell!.new None
a: TransCell! !NoneType
a.set! 1
a: TransCell! !Int
assert a + 1 == 2
```
