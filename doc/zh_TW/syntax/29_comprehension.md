# 推導式

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/29_comprehension.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/29_comprehension.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

List和`[(expr |)? (name <- iterable;)+ (| predicate)?]`,
set和`{(expr |)? (name <- iterable;)+ (| predicate)?}`,
你可以創建一個字典`{(key: value |)? (name <- iterable;)+ (| predicate)?}`.

由`|`分隔的子句的第一部分稱為布局子句，第二部分稱為綁定子句，第三部分稱為條件子句
Either a guard clause or a layout clause can be omitted, but bind clauses cannot be omitted, and a guard clause cannot precede a bind clause.

理解示例

```python
# 布局子句是 i
# 綁定子句是 i <- [0, 1, 2]
assert [i | i <- [0, 1, 2]] == [0, 1, 2]

# If you only want to filter, you can omit the layout clause
# This is same as [0, 1, 2].iter().filter(i -> i % 2 == 0).into_array()
assert [i <- [0, 1, 2] | i % 2 == 0] == [0, 2]

# layout clause: i / 2
# bind clause: i <- 0..2
assert [i/2 | i <- 0..2] == [0.0, 0.5, 1.0]

# 布局子句是 (i, j)
# 綁定子句 i <- 0..2, j <- 0..2
# 保護子句是 (i + j) % 2 == 0
assert [(i, j) | i <- 0..2; j <- 0..2 | (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]

assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

## 篩子類型

與推導類似的是篩類型。篩子類型是以`{Name: Type | Predicate}`創建的(枚舉類型)
refinement類型的情況下，只能指定一個Name，不能指定布局(但是如果是tuple類型可以處理多個值)，Predicate可以在編譯時計算，即 ，只能指定一個常量表達式

```python
Nat = {I: Int | I >= 0}
# 如果謂詞表達式只有and，可以替換為:
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./29_pattern_matching.md'>上一頁</a> | <a href='./31_spread_syntax.md'>下一頁</a>
</p>
