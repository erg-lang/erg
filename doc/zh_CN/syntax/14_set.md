# Set

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/14_set.md%26commit_hash%3Db07c17708b9141bbce788d2e5b3ad4f365d342fa)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/14_set.md&commit_hash=b07c17708b9141bbce788d2e5b3ad4f365d342fa)

一个Set代表一个集合，它在结构上是一个重复的无序数组

```python
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # 重复的被自动删除
assert {1, 2} == {2, 1}
```

它也可以用类型和长度来声明

```python
a: {Int; 3} = {0, 1, 2} # OK
b: {Int; 3} = {0, 0, 0} # NG，重复的内容被删除，长度也会改变
# [
TypeError: the type of b is mismatched
expected:  Set(Int, 3)
but found: Set({0, }, 1)
]# 
```

此外，只有实现`Eq`跟踪的对象才能成为集合的元素

因此，不可能使用Floats等作为集合元素

```python,compile_fail
d = {0.0, 1.0} # NG
# [
1│ d = {0.0, 1.0}
        ^^^^^^^^
TypeError: the type of _ is mismatched:
expected:  Eq(Float)
but found: {0.0, 1.0, }
]# 
```

Set可以执行集合操作

```python
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

Set是同质集合。为了使不同类的对象共存，它们必须同质化

```python
s: {Int or Str} = {"a", 1, "b", -1}
```

## Sets为类型
Sets也可以被视为类型。这种类型称为 _枚举类型_

```python
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

Set的元素直接是类型的元素
请注意，这些Set本身是不同的

```python
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./13_record.md'>上一页</a> | <a href='./15_type.md'>下一页</a>
</p>