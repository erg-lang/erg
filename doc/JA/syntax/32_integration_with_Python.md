# Pythonとの連携

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/32_integration_with_Python.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/32_integration_with_Python.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## Pythonへのexport

Ergスクリプトをコンパイルすると.pycファイルが生成されますが、これは単純にPythonのモジュールとして読み込むことができます。
ただし、Erg側で非公開に設定した変数はPythonからアクセスできません。

```python
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```

```console
erg --compile foo.er
```

```python
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
```

```python
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```

```python
foo = pyimport "foo"
assert foo.bar(1) in Int
```

これは、実行時に型チェックを行うことで型安全性を担保しています。`declare`関数は概ね以下のように動作します。

```python
declare|S: Subroutine| sub!: S, T =
    # 実は、=>はブロックの副作用がなければ関数にキャストできる
    x =>
        assert x in T.Input
        y = sub!(x)
        assert y in T.Output
        y
```

これは実行時オーバーヘッドとなるので、PythonスクリプトをErgの型システムで静的に型解析するプロジェクトが計画されています。

<p align='center'>
    <a href='./31_pipeline.md'>Previous</a> | <a href='./33_package_system.md'>Next</a>
</p>
