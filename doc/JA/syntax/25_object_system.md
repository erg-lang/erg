# Object(対象体)

変数に代入できる全てのデータ。`Object`クラスの持つ属性は以下の通り。

* `.__repr__`: オブジェクトの(リッチでない)文字列表現を返す
* `.__sizeof__`: オブジェクトのサイズ(ヒープ確保分含む)を返す
* `.__dir__`: オブジェクトの属性を一覧にして返す
* `.__hash__`: オブジェクトのハッシュ値を返す
* `.__getattribute__`: オブジェクトの属性を取得して返す
* `.clone`: オブジェクトのクローン（メモリ上に独立な実体を持つ）を生成して返す
* `.copy`: オブジェクトのコピー（メモリ上で同じものをさす）を返す

## Record(レコード)

レコードリテラル(`{attr = value; ...}`)で生成されるオブジェクト。
このオブジェクトは`.clone`や`.__sizeof__`などの基本的なメソッドを持つ。

```erg
obj = {.x = 1}
assert obj.x == 1

obj2 = {...x; .y = 2}
assert obj2.x == 1 and obj2.y == 2
```

## Attribute(属性)

オブジェクトと関連付けられたオブジェクト。特に自身(`self`)を暗黙の第一引数にとるサブルーチン属性はメソッド(method)と呼ばれる。

```erg
# private_attrには`.`がないことに注意
record = {.public_attr = j; private_attr = 2; .method = self -> self.i + 1}
record.public_attr == 2
record.private_attr # AttributeError: private_attr is private
assert record.method() == 3
```

## Element(要素)

特定の型に属するオブジェクト(e.g. `1`は`Int`型の要素)。全てのオブジェクトは、少なくとも`{...}`型の要素である。
クラスの要素の場合特にインスタンス(Instance)と呼ぶこともある。

## Subroutine(サブルーチン)

関数またはプロシージャのインスタンスであるオブジェクトを示す(メソッドも含む)。サブルーチンを表すクラスは`Subroutine`である。
より一般に`.__call__`を実装するオブジェクトは`Callable`(呼び出し可能オブジェクト)と呼ばれる。

## Callable(呼び出し可能オブジェクト)

`.__call__`を実装するオブジェクト。`Subroutine`のスーパークラス。

## Type(型)

要求属性を定義し、オブジェクトを共通化するオブジェクト。
大きく分けて多相型(Polymorphic Type)と単相型(Monomorphic Type)の2つがある。典型的な単相型は`Int`, `Str`などで、多相型には`Option Int`, `[Int; 3]`などがある。
さらにオブジェクトの状態変更をするメソッドを定義した型は可変型(Mutable type)と呼ばれ、可変な属性に`!`をつける必要がある(e.g. 動的配列: `[T; !_]`)。

## Class(クラス)

`.__new__`, `.__init__`メソッドなどを持つ型。クラスベースのオブジェクト指向を実現する。

## Function(関数、写像)

外部変数(静的変数除く)のread権限はあるが、外部変数のread/write権限がないサブルーチン。つまり、外部に副作用を及ぼせない。
Ergの関数(Function)は副作用を許さないのでPythonのそれとは定義が異なる。

## Procedure(手続)

外部変数のread権限および`self`、静的変数のread/write権限があり、全てのサブルーチンの使用が許可されている。外部に副作用を及ぼせる。

## Method(メソッド)

第一引数に`self`を暗黙的にとるサブルーチン。単なる関数/プロシージャとは別の型となっている。

## Entity(エンティティ)

サブルーチンおよび型ではないオブジェクト。
単相型エンティティ(`1`, `"a"`など)は値オブジェクト、多相型エンティティ(`[1, 2, 3], {"a": 1}`)はコンテナオブジェクトとも呼ばれる。

<p align='center'>
    <a href='./24_module.md'>Previous</a> | <a href='./26_pattern_matching.md'>Next</a>
</p>
