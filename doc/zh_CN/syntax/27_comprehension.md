# Comprehension

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/27_comprehension.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/27_comprehension.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Array 和 `[expr | (name <- iterable)+ (predicate)*]`,
set 和 `{expr | (name <- iterable)+ (predicate)*}`,
你可以创建一个字典 `{key: value | (name <- iterable)+ (predicate)*}`.

由`|`分隔的子句的第一部分称为布局子句(位置子句)，第二部分称为绑定子句(绑定子句)，第三部分称为保护子句(条件子句)
保护子句可以省略，但绑定子句不能省略，保护子句不能在绑定子句之前

理解示例

```python
# 布局子句是 i
# 绑定子句是 i <- [0, 1, 2]
assert [i | i <- [0, 1, 2]] == [0, 1, 2]

# 布局子句是 i / 2
# 绑定子句是 i <- 0..2
assert [i/2 | i <- 0..2] == [0.0, 0.5, 1.0]

# 布局子句是 (i, j)
# 绑定子句 i <- 0..2, j <- 0..2
# 保护子句是 (i + j) % 2 == 0
assert [(i, j) | i <- 0..2; j <- 0..2; (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]

assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

Erg推导式受到 Haskell 的启发，但有一些不同
对于 Haskell 列表推导，变量的顺序会对结果产生影响，但在 Erg 中这并不重要

``` haskell
-- Haskell
[(i, j) | i <- [1..3], j <- [3..5]] == [(1,3),(1,4),(1,5),(2 ,3),(2,4),(2,5),(3,3),(3,4),(3,5)]
[(i, j) | j <- [3..5], i <- [1..3]] == [(1,3),(2,3),(3,3),(1 ,4),(2,4),(3,4),(1,5),(2,5),(3,5)]
```

```python
# Erg
assert [(i, j) | i <- 1..<3; j <- 3..<5] == [(i, j) | j <- 3..<5; i <- 1.. <3]
```

该规范与 Python 的规范相同

```python
# Python
assert [(i, j) for i in range(1, 3) for j in range(3, 5)] == [(i, j) for j in range(3, 5) for i in range(1, 3)]
```

## 筛子类型

与推导类似的是筛类型。 筛子类型是以`{Name: Type | Predicate}`创建的(枚举类型)
sieve类型的情况下，只能指定一个Name，不能指定布局(但是如果是tuple类型可以处理多个值)，Predicate可以在编译时计算，即 ，只能指定一个常量表达式

```python
Nat = {I: Int | I >= 0}
# 如果谓词表达式只有and，可以替换为:
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./26_pattern_matching.md'>上一页</a> | <a href='./28_spread_syntax.md'>下一页</a>
</p>