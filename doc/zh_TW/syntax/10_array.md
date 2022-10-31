# Array

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/10_array.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/10_array.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

數組是最基本的__collection(聚合)__
集合是一個可以在其中包含多個對象的對象

```python
a = [1, 2, 3]
a: [Int; 3] # 類型說明: 分號后的數字為元素個數
# 如果元素個數未知，可以省略
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

通常，數組不能包含不同類型的對象

```python.
[1, "a"] # 類型錯誤: 第一個元素是 Int，但第二個元素是 Str
```

但是，您可以通過像這樣顯式指定類型來繞過限制

```python
[1: Int or Str, "a"]
```

## 切片

一個數組也可以同時取出多個值。這稱為切片

```python
l = [1, 2, 3, 4]
# 與 Python 中的 l[1:3] 相同
assert l[1.. <3] == [2, 3]
assert l[1..2] == [2, 3]
# 與 l[1] 相同
assert l[1..1] == [2]
# 與 Python 中的 l[::2] 相同
assert l[..].step(2) == [2, 4]
```

通過切片獲得的對象是數組的(不可變的)副本

```python
print! Typeof l[1..2] # [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>上一頁</a> | <a href='./11_tuple.md'>下一頁</a>
</p>
