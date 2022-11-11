# タプル

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/11_tuple.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/11_tuple.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

タプルは配列と似ていますが、違う型のオブジェクトを保持できます。
このようなコレクションを非等質なコレクションと呼びます。対して等質なコレクションには配列、セットなどがあります。

```python
t = (1, True, "a")
(i, b, s) = t
assert(i == 1 and b == True and s == "a")
```

タプル`t`は`t.n`の形式でn番目の要素を取り出すことができます。Pythonと違い、`t[n]`ではないことに注意してください。
これは、タプル要素のアクセスはメソッド(配列の`[]`はメソッドです)というより属性に近い(コンパイル時に要素の存在がチェックされる、nによって型が変わりうる)ためです。

```python
assert t.0 == 1
assert t.1 == True
assert t.2 == "a"
```

括弧`()`はネストしないとき省略可能です。

```python
t = 1, True, "a"
i, b, s = t
```

タプルは違う型のオブジェクトを保持できますが、そのかわり配列のようなイテレーションができなくなります。

```python
t: ({1}, {2}, {3}) = (1, 2, 3)
(1, 2, 3).iter().map(x -> x + 1) # TypeError: type ({1}, {2}, {3}) has no method `.iter()`
# すべて同じ型の場合配列と同じように`(T; n)`で表せるが、これでもイテレーションは出来ない
t: (Int; 3) = (1, 2, 3)
assert (Int; 3) == (Int, Int, Int)
```

ただし、非等質なコレクション(タプルなど)はアップキャスト、Intersectionなどによって等質なコレクション(配列など)に変換できます。
これを等質化といいます。

```python
homogenize rule

* (X, Y, Z, ...) can be [T; N] if T :> X, T :> Y, T :> Z, ...
```

```python
t: (Int, Bool, Str) = (1, True, "a") # 非等質
a: [Int or Bool or Str; 3] = [1, True, "a"] # 等質
_a: [Show; 3] = [1, True, "a"] # 等質
_a.iter().map(x -> log x) # OK
t.try_into([Show; 3])?.iter().map(x -> log x) # OK
```

## ユニット

要素が0個のタプルはユニットと言います。ユニットは値ですが、自身の型そのものも指します。

```python
unit = ()
(): ()
```

ユニットはすべてのタプルのスーパークラスです。

```python
() :> (Int; 0)
() :> (Str; 0)
() :> (Int, Str)
...
```

このオブジェクトの使いみちは、引数、戻り値がないプロシージャなどです。Ergのサブルーチンは、必ず引数と戻り値を持つ必要があります。しかしプロシージャなどの場合、副作用を起こすだけで意味のある引数・戻り値がない場合もあります。その際に「意味のない、形式上の値」としてユニットを使うわけです。

```python
p!() =
    # `print!`は意味のある値を返さない
    print! "Hello, world!"
p!: () => () # 引数部分は構文の一部でありタプルではない
```

ただしPythonはこのようなときユニットではなく`None`を使う傾向があります。
Ergでの使い分けとしては、プロシージャなどではじめから意味のある値を返さないことが確定しているときは`()`、要素の取得のように操作が失敗して何も得られなかったときは`None`を返してください。

<p align='center'>
    <a href='./10_array.md'>Previous</a> | <a href='./12_dict.md'>Next</a>
</p>
