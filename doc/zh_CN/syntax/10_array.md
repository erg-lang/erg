# Array

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/10_array.md%26commit_hash%3D603abbd5fa3f8baffe0d614758e1a554705e6732)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/10_array.md&commit_hash=603abbd5fa3f8baffe0d614758e1a554705e6732)

数组是最基本的__collection(聚合)__
集合是一个可以在其中包含多个对象的对象

```python
a = [1, 2, 3]
a: [Int; 3] # 类型说明: 分号后的数字为元素个数
# 如果元素个数未知，可以省略
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

通常，数组不能包含不同类型的对象

```python.
[1, "a"] # 类型错误: 第一个元素是 Int，但第二个元素是 Str
```

但是，您可以通过像这样显式指定类型来绕过限制

```python,compile_fail
[1: Int or Str, "a"]
```

## 切片

一个数组也可以同时取出多个值。这称为切片

```python
l = [1, 2, 3, 4]
# 与 Python 中的 l[1:3] 相同
assert l[1.. <3] == [2, 3]
assert l[1..2] == [2, 3]
# 与 l[1] 相同
assert l[1..1] == [2]
# 与 Python 中的 l[::2] 相同
assert l[..].step(2) == [2, 4]
```

通过切片获得的对象是数组的(不可变的)副本

```python
print! Typeof l[1..2] # [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>上一页</a> | <a href='./11_tuple.md'>下一页</a>
</p>
