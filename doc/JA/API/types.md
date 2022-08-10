# Erg組み込み型一覧

型自体の属性は`.__dict__`の中には格納されていないので、インスタンスからは参照できない

## 汎用型 (Fundamental types)

### Object

* `__dir__`: オブジェクトの持つ属性を配列にして返す(dir関数)
* `__getattribute__`: 属性を取得して返す
* `__hash__`: オブジェクトのハッシュ値を返す
* `__repr__`: オブジェクトの文字列表現(リッチでない/デフォルト実装が存在)
* `__sizeof__`: オブジェクトのサイズ(ヒープに確保された分も含む)を返す

### Show

* `__str__`: オブジェクトの文字列表現(リッチな)を返す

### Fmt

* `__format__`: フォーマットされた文字列を返す

### Doc

* `__doc__`: オブジェクトの説明

### Named

* `__name__`: オブジェクトの名前

### Pickle

* `__reduce__`: Pickleによるオブジェクトのシリアライズを行う
* `__reduce_ex__`: プロトコルバージョンを指定できる__reduce__

## オブジェクトシステム

Trait classはPythonでのABC(抽象基底クラス、インターフェース)に相当
Instanceに属するのは1, True, "aaa"など
ClassはInt, Bool, Strなど

### Type

* `__supers__`: 上位型(`__mro__`は配列だが、こちらはSet)
* `__basicsize__`:
* `__dictoffset__`: Evmではサポートされない
* `__flags__`:
* `__itemsize__`: インスタンスのサイズ(Classでない場合は0)
* `__weakrefoffset__`: Evmではサポートされない
* `__membercheck__`: `ismember(x, T)`と同等
* `__subtypecheck__`: `issubtype(U, T)`と同等、`__subclasshook__`というエイリアスが存在(CPythonとの互換)

### Instance

* `__class__`: インスタンスの生成元クラスを返す(`.new`で生成されたオブジェクトは自動で付く)

### Class

* `__mro__`: メソッド解決用の型配列(自身も入っている、最後は必ずObject)
* `__base__`: ベースとなった型(複数ある場合、`__mro__[1]`)
* `__new__`: インスタンスを生成する
* `__init__`: インスタンスを初期化する
* `__init_subclass__`: インスタンスを初期化する
* `__intstancecheck__`: `MyClass.__instancecheck__(x)`などのように使う、`isinstance(x, MyClass)`と同等
* `__subclasscheck__`: `issubclass(C, MyClass)`と同等

## 演算子

ここで指定されている以外の演算子には特に専用の型はない

### Eq

* `__eq__(self, rhs: Self) -> Bool`: オブジェクトの比較関数(==)
* `__ne__`: オブジェクトの比較関数(!=)、デフォルト実装あり

### Ord

* `__lt__(self, rhs: Self) -> Bool`: オブジェクトの比較関数(<)
* `__le__`: オブジェクトの比較関数(<=)、デフォルト実装あり
* `__gt__`: オブジェクトの比較関数(>)、デフォルト実装あり
* `__ge__`: オブジェクトの比較関数(>=)、デフォルト実装あり

### BinAdd

* `__add__(self, rhs: Self) -> Self`: `+`を実装

### Add R, O

* `__add__(self, rhs: R) -> O`

### BinSub

* `__sub__(self, rhs: Self) -> Self`: `-`を実装

### Sub R, O

* `__sub__(self, rhs: R) -> O`

### BinMul

* `__mul__(self, rhs: Self) -> Self`: `*`を実装
* `__pow__`: `**`を実装(デフォルト実装あり)

### Mul R, O

* `__mul__(self, rhs: R) -> O`
* `__pow__`

### BinDiv

* `__div__(self, rhs: Self) -> Self`: `/`を実装、0によりパニックしてもよい
* `__mod__`: `%`を実装(デフォルト実装あり)

### Div R, O

* `__div__(self, rhs: R) -> O`
* `__mod__`

## 数値型

### Num (= Add and Sub and Mul and Eq)

Complex以外の例として、Vector, Matrix, TensorはNum(Matrix, Tensorの*はそれぞれdot, productと同じ)

### Complex (= Inherit(Object, Impl=Num))

* `imag: Ratio`: 虚部を返す
* `real: Ratio`: 実部を返す
* `conjugate self -> Complex`: 共役複素数を返す

### Float (= Inherit(FloatComplex, Impl=Num))

### Ratio (= Inherit(Complex, Impl=Num))

* `numerator: Int`: 分子を返す
* `denominator: Int`: 分母を返す

### Int (= Inherit Ratio)

### Nat (= Inherit Int)

* `times!`: self回procを実行する

## その他基本型

### Bool

* `__and__`:
* `__or__`:
* `not`:

## Str (<: Seq)

* `capitalize`
* `chomp`: 改行文字を除去
* `isalnum`:
* `isascii`:
* `isalpha`:
* `isdecimal`:
* `isdight`:
* `isidentifier`
* `islower`
* `isnumeric`
* `isprintable`
* `isspace`
* `istitle`
* `isupper`
* `lower`
* `swapcase`
* `title`
* `upper`

## その他

### Bit

* `from_bytes`: Bytesから変換
* `to_bytes`: Bytesへ変換(長さ(length)、エンディアン(byteorder)を指定)
* `bit_length`: Bit長を返す

### Iterable T

`Iterator`自体の型ではない点に注意。`Nat`は`Iterable`だが`Nat.next()`とはできず、`Nat.iter().next()`とする必要がある。

* `iter`: Iteratorを生成する。

### Iterator T

NatやRangeはIteratorを持っているので、`Nat.iter().map n -> n**2`, `(3..10).iter().fold (sum, n) -> sum + n*2`などが可能。
allやanyは使用後破壊されるので副作用なし。これらは副作用のない`next`を使って実装されていることになっているが、実行効率のため内部的には`Iterator!.next!`を使っている。

* `next`: 先頭の要素と残りのIteratorを返す。
* `all`
* `any`
* `filter`
* `filter_map`
* `find`
* `find_map`
* `flat_map`
* `flatten`
* `fold`
* `for_each`
* `map`
* `map_while`
* `nth`
* `pos`
* `take`
* `unzip`
* `zip`

### Iterator! T = Iterator T and ...

* `next!`: 先頭の要素を取り出す。

## SizedIterator T = Iterator T and ...

有限の要素を走査するIterator。

* `len`:
* `chain`:
* `count`:
* `is_empty`:
* `rev`:
* `next_back`:
* `nth_back`:
* `rfind`:
* `rfold`:
* `sum`:
* `max`:
* `min`:

## Seq T = SizedIterable T and ...

* `concat`: 2つのSeqを結合する
* `__getitem__`: `[]`によるアクセスと同等(なければパニック)
* `get`: __getitem__と違ってOptionで返す
* `maketrans`: 置換テーブルをつくる(スタティックメソッド)
* `replace`: 置換する
* `translate`: 置換テーブルにそって置換する
* `insert`: idx番目に追加
* `remove`: idx番目を取り出し
* `prepend`: 先頭に追加
* `dequeue`: 先頭を取り出し
* `push`: 最後尾に追加
* `pop`: 最後尾を取り出し
* `dedup`: 連続する値を削除
* `uniq`: 重複する要素を削除(sort |> dedupで実装されるため、順番が変わる可能性がある)
* `swap`: 要素を入れ替え
* `reverse`: 要素を反転
* `sort`: 要素をソート
* `first`:
* `last`:

### Seq! T (= Seq T and ...)

* `__setitem__!`:
* `__delitem__!`:
* `insert!`: idx番目に追加
* `remove!`: idx番目を取り出し
* `prepend!`: 先頭に追加
* `dequeue!`: 先頭を取り出し
* `push!`: 最後尾に追加
* `pop!`: 最後尾を取り出し
* `dedup!`: 連続する値を削除
* `uniq!`: 重複する要素を削除(sort! |> dedup!で実装されるため、順番が変わる可能性がある)
* `swap!`: 要素を入れ替え
* `reverse!`: 要素を反転
* `set!`
* `sort!`: 要素をソート
* `translate!`
