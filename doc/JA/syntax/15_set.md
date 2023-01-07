# セット

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/15_set.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/15_set.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

セットは集合を表し、データ構造的には重複、順序のない配列です。

```python
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # 重複は自動で削除される
assert {1, 2} == {2, 1}
```

型や長さを指定して宣言することもできます。

```python
a: {Int; 3} = {0, 1, 2} # OK
b: {Int; 3} = {0, 0, 0} # NG、重複が削除されて長さが変わる
# TypeError: the type of b is mismatched
# expected:  Set(Int, 3)
# but found: Set({0, }, 1)
```

また、`Eq`トレイトが実装されているオブジェクトのみが集合の要素になれます。

そのため、Floatなどを集合の要素として使用することはできません。

```python,compile_fail
d = {0.0, 1.0} # NG
# Error[#1366]: File <stdin>, line 1, <module>::d
#
# 1 | d = {0.0, 1.0}
#   :      --------
#   :             |- expected: Eq
#   :             |- but found: {0.0f, 1.0f, }
#   :             `- Float has no equivalence relation defined. you should use l == R instead of l - r <= Float.EPSILON
#
# TypeError: the type of _ is mismatched
```

セットは集合演算を行えます。

```python
assert 1 in {1, 2, 3}
assert not 1 in {}
assert {1} or {2} == {1, 2}
assert {1, 2} and {2, 3} == {2}
assert {1, 2} not {2} == {1}
```

セットは等質なコレクションです。別のクラスのオブジェクトを共存させるためには、等質化させなくてはなりません。

```python
s: {Int or Str} = {"a", 1, "b", -1}
```

## 型としてのセット

セットは型としても扱えます。このような型は __列挙型(Enum type)__ と呼ばれます。

```python
i: {1, 2, 3} = 1
assert i in {1, 2, 3}
```

セットの要素がそのまま型の要素になります。
セット自身は違うことに注意が必要です。

```python
mut_set = {1, 2, 3}.into {Int; !3}
mut_set.insert!(4)
```

<p align='center'>
    <a href='./14_record.md'>Previous</a> | <a href='./16_type.md'>Next</a>
</p>
