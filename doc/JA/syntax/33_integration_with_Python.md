# Pythonとの連携

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/32_integration_with_Python.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/33_integration_with_Python.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

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

Python標準ライブラリにあるAPIはすべてErg開発チームにより型が指定されています。

```python
time = pyimport "time"
time.sleep! 1
```

## ユーザースクリプトの型指定

Pythonの`foo`モジュールに型を付ける`foo.d.er`ファイルを作成します。
Python側でのtype hintは100%の保証にならないので無視されます。

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

Pythonの関数はすべてプロシージャとして、クラスはすべて可変クラスとしてしか登録できないことに注意してください。

```python
foo = pyimport "foo"
assert foo.bar!(1) in Int
```

これは、実行時に型チェックを行うことで型安全性を担保しています。チェック機構は概念的には以下のように動作します。

```python
decl_proc proc!: Proc, T =
    x =>
        assert x in T.Input
        y = proc!(x)
        assert y in T.Output
        y
```

これは実行時オーバーヘッドとなるので、PythonスクリプトをErgの型システムで静的に型解析するプロジェクトが計画されています。

<p align='center'>
    <a href='./32_pipeline.md'>Previous</a> | <a href='./34_package_system.md'>Next</a>
</p>
