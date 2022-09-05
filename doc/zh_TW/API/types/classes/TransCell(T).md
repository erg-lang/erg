# TransCell! T: Type!

它是一個單元格，其內容可以針對每個模具進行更改。 由于它是T類型的子類型，因此它也表現為T類型
當它在初始化時輸入T時很有用，并且在某個點之后總是輸入U

```python
a = TransCell!.new None
a: TransCell! !NoneType
a.set! 1
a: TransCell! !Int
assert a + 1 == 2
```
