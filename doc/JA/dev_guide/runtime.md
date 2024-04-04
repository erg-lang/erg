# Pythonで実装されているモジュール

## [_erg_array.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_array.py)

`list`のラッパーである`List`クラスを定義します。

## [_erg_bool.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_bool.py)

`Nat`のラッパー(`bool`ではないことに注意)である`Bool`クラスを定義します。

## [_erg_bytes.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_bytes.py)

## [_erg_control.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_control.py)

`for!`、`if!`などの制御構造を実現する関数を定義します。

## [_erg_converters.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_convertors.py)

`int`や`str`などのコンストラクタを定義します。これらのコンストラクタは現状、失敗時に`None`を返します。

## [_erg_dict.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_dict.py)

`dict`のラッパーである`Dict`クラスを定義します。

## [_erg_float.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_float.py)

## [_erg_in_operator.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_in_operator.py)

`in`演算子の実装を定義します。Ergの`in`演算子はPythonの`in`演算子の機能に加えて、型の包含判定も行います。
例えば`1 in Int`、``[1, 2] in [Int; 2]``などが可能です。

## [_erg_int.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_int.py)

## [_erg_mutate.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_mutate_operator.py)

`!`演算子の実装を定義します。`!`演算子はオブジェクトを可変化します。例えば、`Int`を`IntMut`(`Int!`)に変換します。
これは、実際は`mutate`メソッドを呼び出しているだけです。

## [_erg_nat.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_nat.py)

## [_erg_range.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_range.py)

`1..3`などで現れる範囲オブジェクトを定義します。
これはPythonの`range`で返される`range`オブジェクトとは全く異なっており、セマンティクス的にはどちらかというとRustの`Range`に近いです。
`Int`だけでなく整列可能なオブジェクト全般に対して使用できます。

## [_erg_result.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_result.py)

エラーの基底クラスである`Error`を定義します。

## [_erg_set.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_set.py)

## [_erg_std_prelude.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_std_prelude.py)

Ergランタイムのエントリポイントです。

## [_erg_str.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_str.py)

# Ergで実装されているモジュール

## [abc.er](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/abc.er)

トレイトを実装します。未実装の機能が使われており、コンパイル出来ません。

## [semver.er](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/semver.er)

セマンティックバージョンを取り扱うためのモジュールです。

## consts

### [consts/physics](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/consts/physics.er)

よく使われる物理定数を定義します。
