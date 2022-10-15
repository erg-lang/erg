# 推導式

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/27_comprehension.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/27_comprehension.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Array 和 `[expr | (name <- iterable)+ (predicate)*]`,
set 和 `{expr | (name <- iterable)+ (predicate)*}`,
你可以創建一個字典 `{key: value | (name <- iterable)+ (predicate)*}`.

由`|`分隔的子句的第一部分稱為布局子句(位置子句)，第二部分稱為綁定子句(綁定子句)，第三部分稱為保護子句(條件子句)
保護子句可以省略，但綁定子句不能省略，保護子句不能在綁定子句之前

理解示例

```python
# 布局子句是 i
# 綁定子句是 i <- [0, 1, 2]
assert [i | i <- [0, 1, 2]] == [0, 1, 2]

# 布局子句是 i / 2
# 綁定子句是 i <- 0..2
assert [i/2 | i <- 0..2] == [0.0, 0.5, 1.0]

# 布局子句是 (i, j)
# 綁定子句 i <- 0..2, j <- 0..2
# 保護子句是 (i + j) % 2 == 0
assert [(i, j) | i <- 0..2; j <- 0..2; (i + j) % 2 == 0] == [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)]

assert {i % 2 | i <- 0..9} == {0, 1}
assert {k: v | k <- ["a", "b"]; v <- [1, 2]} == {"a": 1, "b": 2}
```

Erg推導式受到 Haskell 的啟發，但有一些不同
對于 Haskell 列表推導，變量的順序會對結果產生影響，但在 Erg 中這并不重要

``` haskell
-- Haskell
[(i, j) | i <- [1..3], j <- [3..5]] == [(1,3),(1,4),(1,5),(2 ,3),(2,4),(2,5),(3,3),(3,4),(3,5)]
[(i, j) | j <- [3..5], i <- [1..3]] == [(1,3),(2,3),(3,3),(1 ,4),(2,4),(3,4),(1,5),(2,5),(3,5)]
```

```python
# Erg
assert [(i, j) | i <- 1..<3; j <- 3..<5] == [(i, j) | j <- 3..<5; i <- 1.. <3]
```

該規范與 Python 的規范相同

```python
# Python
assert [(i, j) for i in range(1, 3) for j in range(3, 5)] == [(i, j) for j in range(3, 5) for i in range(1, 3)]
```

## 篩子類型

與推導類似的是篩類型。篩子類型是以`{Name: Type | Predicate}`創建的(枚舉類型)
sieve類型的情況下，只能指定一個Name，不能指定布局(但是如果是tuple類型可以處理多個值)，Predicate可以在編譯時計算，即 ，只能指定一個常量表達式

```python
Nat = {I: Int | I >= 0}
# 如果謂詞表達式只有and，可以替換為:
# Nat2D = {(I, J): (Int, Int) | I >= 0; J >= 0}
Nat2D = {(I, J): (Int, Int) | I >= 0 and J >= 0}
```

<p align='center'>
    <a href='./26_pattern_matching.md'>上一頁</a> | <a href='./28_spread_syntax.md'>下一頁</a>
</p>