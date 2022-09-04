# TransCell! T: Type!

中身を型ごと変えられるセルです。T型のサブタイプとなるので、T型としても振る舞います。
初期化時点ではT型で、ある時点以降はずっとU型、といった場合に便利です。

```python
a = TransCell!.new None
a: TransCell! !NoneType
a.set! 1
a: TransCell! !Int
assert a + 1 == 2
```
