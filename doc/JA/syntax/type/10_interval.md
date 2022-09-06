# Interval Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/10_interval.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/10_interval.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

`Range`オブジェクトの最も基本的な使い方は、イテレータとしての使用です。

```python
for! 0..9, i =>
    print! i
```

Pythonと違い、末尾の数字は含まれることに注意してください。

しかし、`Range`オブジェクトの使い道はこれだけではありません。型としても使うことが出来ます。このような型を区間型(Interval type)と呼びます。

```python
i: 0..10 = 2
```

`Nat`型は`0..<Inf`と等価な型で、`Int`と`Ratio`型は`-Inf<..<Inf`と等価な型です。
`0..<Inf`は`0.._`と書くことも出来ます。`_`は、`Int`型の任意のインスタンスを意味します。

イテレータとしても使えるため、`10..0`などのように逆順で指定することも出来ますが、
`<..`, `..<`, `<..<`の向きは逆転できません。

```python
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # Syntax error
e = 10..<0 # Syntax error
f = 10<..<0 # Syntax error
```

範囲演算子(range operator)は、`Ord`な不変型であるならば数値以外の型にも使用できます。

```python
Alphabet = "A".."z"
```
