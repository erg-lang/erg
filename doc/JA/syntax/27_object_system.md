# 対象体

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/26_object_system.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/26_object_system.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

変数に代入できる全てのデータです。`Object`クラスの持つ属性は以下の通りです。

* `.__repr__`: オブジェクトの(リッチでない)文字列表現を返します
* `.__sizeof__`: オブジェクトのサイズ(ヒープ確保分含む)を返します
* `.__dir__`: オブジェクトの属性を一覧にして返します
* `.__hash__`: オブジェクトのハッシュ値を返します
* `.__getattribute__`: オブジェクトの属性を取得して返します
* `.clone`: オブジェクトのクローン(メモリ上に独立な実体を持つ)を生成して返します
* `.copy`: オブジェクトのコピー(メモリ上で同じものをさす)を返します

## レコード

レコードリテラル(`{attr = value; ...}`)で生成されるオブジェクトです。
このオブジェクトは`.clone`や`.__sizeof__`などの基本的なメソッドを持ちます。

```python
obj = {.x = 1}
assert obj.x == 1

obj2 = {*x; .y = 2}
assert obj2.x == 1 and obj2.y == 2
```

## 属性

オブジェクトと関連付けられたオブジェクトです。特に自身(`self`)を暗黙の第一引数にとるサブルーチン属性はメソッド(method)と呼ばれます。

```python
# private_attrには`.`がないことに注意が必要になる
record = {.public_attr = j; private_attr = 2; .method = self -> self.i + 1}
record.public_attr == 2
record.private_attr # AttributeError: private_attr is private
assert record.method() == 3
```

## 要素

特定の型に属するオブジェクト(e.g. `1`は`Int`型の要素)です。全てのオブジェクトは、少なくとも`{=}`型の要素です。
クラスの要素の場合特にインスタンス(Instance)と呼ぶこともあります。

## サブルーチン

関数またはプロシージャのインスタンスであるオブジェクトを示す(メソッドも含む)。サブルーチンを表すクラスは`Subroutine`です。
より一般に`.__call__`を実装するオブジェクトは`Callable`(呼び出し可能オブジェクト)と呼ばれます。

## 呼び出し可能オブジェクト

`.__call__`を実装するオブジェクトです。`Subroutine`のスーパークラスでもあります。

## 型

要求属性を定義し、オブジェクトを共通化するオブジェクトです。
大きく分けて多相型(Polymorphic Type)と単相型(Monomorphic Type)の2つがあります。典型的な単相型は`Int`, `Str`などで、多相型には`Option Int`, `[Int; 3]`などがあります。
さらにオブジェクトの状態変更をするメソッドを定義した型は可変型(Mutable type)と呼ばれ、可変な属性に`!`をつける必要があります(e.g. 動的配列: `[T; !_]`)。

## クラス

`.__new__`, `.__init__`メソッドなどを持つ型です。クラスベースのオブジェクト指向を実現します。

## 関数

外部変数(静的変数除く)のread権限はありますが、外部変数のread/write権限がないサブルーチンです。つまり、外部に副作用を及ぼせません。
Ergの関数(Function)は副作用を許さないので、Pythonのそれとは定義が異なります。

## 手続き

外部変数のread権限および`self`、静的変数のread/write権限があり、全てのサブルーチンの使用が許可されています。外部に副作用を及ぼせます。

## メソッド

第一引数に`self`を暗黙的にとるサブルーチンです。単なる関数/プロシージャとは別の型となっています。

## エンティティ

サブルーチンおよび型ではないオブジェクトです。
単相型エンティティ(`1`, `"a"`など)は値オブジェクト、多相型エンティティ(`[1, 2, 3], {"a": 1}`)はコンテナオブジェクトとも呼ばれます。

<p align='center'>
    <a href='./26_module.md'>Previous</a> | <a href='./28_pattern_matching.md'>Next</a>
</p>
