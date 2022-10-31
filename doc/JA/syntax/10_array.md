# 配列

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/10_array.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/10_array.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

配列はもっとも基本的な __コレクション(集約)__ です。
コレクションとは、内部にオブジェクトを複数保持できるオブジェクトのことです。

```python
a = [1, 2, 3]
a: [Int; 3] # 型指定: セミコロンの後の数字は要素数
# 要素数がわからない場合は省略できる
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

配列には、原則として違う型のオブジェクトを入れることはできません。

```python
[1, "a"] # TypeError: 1st element is Int, but 2nd element is Str
```

しかし、このように要素の型を明示的に指定すると制限を回避できます。

```python
[1: Int or Str, "a"]
```

## スライス

配列は、複数の値をまとめて取り出すこともできます。これをスライスと呼びます。

```python
l = [1, 2, 3, 4]
# Pythonのl[1:3]に相当
assert l[1..<3] == [2, 3]
assert l[1..2] == [2, 3]
# l[1]と同じ
assert l[1..1] == [2]
# Pythonのl[::2]に相当
assert l[..].step(2) == [2, 4]
```

スライスで得られるオブジェクトは配列の(不変)参照です。

```python
print! Typeof l[1..2] # Ref [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>Previous</a> | <a href='./11_tuple.md'>Next</a>
</p>
