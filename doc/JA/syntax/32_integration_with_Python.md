# Pythonとの連携

Pythonから取り込んだオブジェクトはデフォルトですべて`Object`型になります。このままでは比較もできないので、型の絞り込みを行う必要があります。

## 標準ライブラリの型指定

Python標準ライブラリにあるAPIはすべてErg開発チームにより型が指定されています。

```erg
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

```erg
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```

```erg
foo = pyimport "foo"
assert foo.bar(1) in Int
```

これは、実行時に型チェックを行うことで型安全性を担保しています。`declare`関数は概ね以下のように動作します。

```erg
declare<S: Subroutine> sub!: S, T =
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
