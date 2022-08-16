# Spread assignment (展開代入)

分解代入において、変数の前に`...`を置くと残りの要素を全てその変数に展開できます。これを展開代入と呼びます。

```erg
[x, ...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## Extract assignment (抽出代入)

`...`のあとに何も書かない場合、残りの要素は無視して代入されます。このタイプの展開代入を特に抽出代入と呼びます。
抽出代入は、モジュールやレコード内にある特定の属性をローカルに持ってくる際に便利な構文です。

```erg
{sin; cos; tan; ..} = import "math"
```

このようにすると、以降はローカルで`sin, cos, tan`が使用できます。

レコードでも同じようにできます。

```erg
record = {x = 1; y = 2}
{x; y; ...} = record
```

全て展開したい場合は`{*} = record`とします。OCamlなどでいう`open`です。

```erg
record = {x = 1; y = 2}
{*} = record
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>Previous</a> | <a href='./29_decorator.md'>Next</a>
</p>
