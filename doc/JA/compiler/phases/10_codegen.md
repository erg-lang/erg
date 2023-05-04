# コード生成

Ergスクリプトは、デフォルトではpycファイルに変換されて実行されます。つまり、Pythonスクリプトではなく[Pythonバイトコード](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/doc/JA/python/bytecode_instructions.md)として実行されます。
pycファイルは構文糖が剥がされ(phase 8)、依存関係の結合(phase 9)されたHIRから生成されます。
処理は[`PyCodeGenerator`](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/codegen.rs#L160)が行います。この構造体は`HIR`を受け取って`CodeObj`を返します。
`CodeObj`はPythonのCodeオブジェクトに対応し、実行する命令列や静的領域のオブジェクト、その他様々なメタデータを持ちます。`Code`オブジェクトはPythonインタプリタから見るとスコープを表すオブジェクトです。トップレベルのスコープを表す`Code`が、実行に必要な全ての情報を持つことになります。`CodeObj`は[dump_as_pyc](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/ty/codeobj.rs#L378)メソッドでバイナリ化され、pycファイルに書き出されます。

# Pythonに存在しない機能

## Ergランタイム

ErgはPythonインタプリタ上で動作しますが、Pythonとは様々なセマンティクスの違いがあります。
幾つかの機能はコンパイラがより低次の機能に脱糖することで実現されますが、ランタイムで実現するしかないものもあります。

組み込み型のPythonには存在しないメソッドなどがその例です。
Pythonの組み込みには`Nat`型は存在しませんし、`times!`メソッドも存在しません。
このようなメソッドは、Pythonの組み込み型をラップした新しい型を作ることで実現しています。

それらの型は[](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std)に置かれています。
生成されるバイトコードはまず[`_erg_std_prelude.py`をimport](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/codegen.rs#L3113)します。このモジュールはErgランタイムの提供する型、関数をre-exportします。

## Record

レコードは、Pythonの`namedtuple`で実現されています。

## トレイト

トレイトは、実体としてはPythonのABC(抽象基底クラス)で実現されています。
といっても、Ergのトレイトは実行時にはほとんど意味を持ちません。

## match

パターンマッチは大抵、型判定と代入動作の組み合わせに還元されます。これはコンパイルの比較的早い段階で行われます。

```erg
i, [j, *k] = 1, [2, 3, 4]
```

↓

```erg
_0 = 1, [2, 3]
i = _0[0]
_1 = _0[1]
j = _1[0]
k = _1[1:]
```

しかし、実行時まで遅延するものもあります。

```erg
x: Int or Str
match x:
    i: Int -> ...
    s: Str -> ...
```

このパターンマッチは実行時の判定を必要とします。この判定は[`in_operator`](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_in_operator.py#L6)が行います。

したがって、上のコードを脱糖すると以下のようになります。網羅性検査はコンパイル時に行われます。

```erg
if in_operator(x, Int):
    ...
else:
    ...
```

## 制御構造をなす関数

`for!`や`if!`などPythonの制御構造に対応する関数は、最適化状況によって実体が変わります。通常は最適化が行えて、専用のバイトコード命令に還元されます。

```erg
for! [a, b], i =>
    ...
```

↓

```pyc
LOAD_NAME 0(a)
LOAD_NAME 1(b)
BUILD_LIST 2
GET_ITER
FOR_ITER ...
STORE_NAME 2(i)
...
```

これは関数呼び出しよりも効率的です。しかし以下のように、最適化が行えない場合もあります。

```erg
f! = [for!, ...].choice!()

f! [1, 2], i =>
    ...
```

このような場合は実体を持つ関数として扱わなくてはなりません。関数は[_erg_control.py](https://github.com/erg-lang/erg/blob/d1dc1e60e7d4e3333f80ed23c5ead77b5fe47cb2/crates/erg_compiler/lib/std/_erg_control.py)に定義されています。
