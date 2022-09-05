# ジェネレータ


ジェネレータは、ブロック中で`yield!`プロシージャを使う特殊なプロシージャです。

```python
g!() =
    yield! 1
    yield! 2
    yield! 3
```

`yield!`はサブルーチンのブロックで定義されるプロシージャで、`self!.yield!`を呼び出します。
これは`return`と同じく渡された値を戻り値として返すものですが、その時点でのブロックの実行状態を保存し、もう一度呼び出された際に続きから実行するという特徴があります。
ジェネレータはプロシージャでありながらイテレータでもあります。Pythonのジェネレータはイテレータを生成する関数ですが、Ergは直接イテレートします。プロシージャ自体は一般に可変オブジェクトではありません(`!`が付かない)が、ジェネレータは実行ごとに自身の内容が変わる得るので可変オブジェクトです。

```python
# Generator! < Proc
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

Pythonスタイルのジェネレータは以下のようにして定義できます。

```python
make_g() = () =>
    yield! 1
    yield! 2
    yield! 3
make_g: () => Generator!((), Int)
```

<p align='center'>
    <a href='./33_package_system.md'>Previous</a> | Next
</p>
