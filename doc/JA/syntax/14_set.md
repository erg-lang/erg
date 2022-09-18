# セット

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/14_set.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/14_set.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

セットは集合を表し、データ構造的には重複、順序のない配列です。

```python
assert Set.from([1, 2, 3, 2, 1]) == {1, 2, 3}
assert {1, 2} == {1, 1, 2} # 重複は自動で削除される
assert {1, 2} == {2, 1}
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
    <a href='./13_record.md'>Previous</a> | <a href='./15_type.md'>Next</a>
</p>
