# Pythonとの連携

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_integration_with_Python.md%26commit_hash%3D0150fcc2b15ec6b4521de2b84fa42174547c2339)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_integration_with_Python.md&commit_hash=0150fcc2b15ec6b4521de2b84fa42174547c2339)

## Pythonへのexport

Ergスクリプトをコンパイルすると.pycファイルが生成されますが、これは単純にPythonのモジュールとして読み込むことができます。
ただし、Erg側で非公開に設定した変数はPythonからもアクセスできません。

```python
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```

```console
erg --compile foo.er
```

```python,checker_ignore
import foo

print(foo.public)
print(foo.private) # AttributeError:
```

## Pythonからのimport

Pythonから取り込んだオブジェクトはデフォルトですべて`Object`型になります。このままでは比較もできないので、型の絞り込みを行う必要があります。

## 標準ライブラリの型指定

Python標準ライブラリにあるAPIは、すべてErg開発チームにより予め型が指定されています。なので、`pyimport`でそのまま呼び出すことが出来ます。

```python
time = pyimport "time"
time.sleep! 1
```

## ユーザースクリプトの型指定

Pythonスクリプトの型ヒント(type hint)をErgは関知しません。

Pythonの`foo`モジュールに型を付ける`foo.d.er`ファイルを作成します。

```python
# foo.py
X = ...
def bar(x):
    ...
def baz():
    ...
class C:
    ...
```

```python
# foo.d.er
.X: Int
.bar!: Int => Int
.foo! = baz!: () => Int # aliasing
.C!: Class
```

`d.er`内では宣言と定義(エイリアシング)以外の構文は使えません。

Python側での識別子がErgでは有効な識別子ではない場合、シングルクォーテーション(`'`)で囲むことでエスケープできます。

## オーバーロード

Pythonの型付けだけで使える特殊な型として、オーバーロード型があります。これは、複数の型を受け取ることができる型です。

```python
f: (Int -> Str) and (Str -> Int)
```

オーバーロード型はサブルーチン型のintersection(`and`)を取ることで宣言できます。`or`ではないことに注意してください。

こうすると、引数の型によって戻り値の型が変わる関数を宣言できます。

```python
f(1): Str
f("1"): Int
```

型判定は左から順に照合され、最初にマッチしたものが適用されます。

このような多相はアドホック多相と呼ばれ、型変数とトレイト境界を用いるErgの多相とは異なるものです。アドホック多相は一般的にはあまり推奨されませんが、Pythonのコードでは普遍的に使われているので、必要悪として存在します。

オーバーロード型の引数型は部分型関係にあっても良く、引数数が違っていいても良いですが、同じ型であってはいけません。すなわち、return type overloadingは許可されません。

```python
# OK
f: (Nat -> Str) and (Int -> Int)
f: ((Int, Int) -> Str) and (Int -> Int)
```

```python,compile_fail
# NG
f: (Int -> Str) and (Int -> Int)
```

## トレイト実装宣言

クラスに対してトレイトの実装とトレイトメンバーの宣言を行う場合、以下のように記述します([numpy.NDListの型宣言](https://github.com/erg-lang/erg/blob/main/crates/erg_compiler/lib/external/numpy.d/__init__.d.er)より抜粋)。

```erg
.NDList = 'ndarray': (T: Type, Shape: [Nat; _]) -> ClassType
...
.NDList(T, S)|<: Add .NDList(T, S)|.
    Output: {.NDList(T, S)}
    __add__: (self: .NDList(T, S), other: .NDList(T, S)) -> .NDList(T, S)
```

## 注意点

現在のところ、Ergはこの型宣言の内容を無条件に信用します。すなわち、実際にはInt型の変数でもStr型として宣言する、副作用のあるサブルーチンでも関数として宣言する、などができてしまいます。

また、自明な型付けでも型宣言を省略できないのは面倒なので、[PythonスクリプトをErgの型システムで静的に型解析するプロジェクト](https://github.com/mtshiba/pylyzer)が進められています。

<p align='center'>
    <a href='./33_pipeline.md'>Previous</a> | <a href='./35_package_system.md'>Next</a>
</p>
