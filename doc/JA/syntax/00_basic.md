# 基本事項


> __Warning__: 本ドキュメントは未完成です。校正(文体、正しいリンクが張られているか、など)がなされていません。また、Ergの文法はバージョン0.*の間に破壊的変更が加えられる可能性があり、それに伴うドキュメントの更新が追いついていない可能性があります。予めご了承ください。
> また、本ドキュメントの誤りを見つけた場合は、[こちらのフォーム](https://forms.gle/HtLYRfYzWCAaeTGb6)または[GitHubリポジトリ](https://github.com/mtshiba/TheErgBook/issues/new)から修正の提案をしていただけると幸いです。

本ドキュメントは、Ergの基本文法について解説するものです。[標準API](../API/index.md)や[Ergコントリビューター向けの内部資料](../dev_guide/index.md)は別のディレクトリに置かれています。

## Hello, World&excl;

まずは恒例の、Hello Worldを行いましょう。

```python
print!("Hello, World!")
```

Pythonや同系統の言語とほぼ同じです。目を引くのは`print`の後に付く`!`ですが、これの意味はおいおい説明します。
また、Ergでは解釈に紛れのない限り括弧`()`を省略することが出来ます。
括弧の省略ができるのはRubyと似ていますが、複数の解釈ができる括弧省略はできませんし、また引数が0個のときもPythonと同じく`()`の省略が出来ません。

```python
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OK, but this does not mean to call, simply to get `print!` as a callable object

print! f x # OK, interpreted as `print!(f(x))`
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print!(f x, y) # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print! f(x, g y, z) # NG, can be taken to mean either `print!(x, g(y), z)` or `print!(x, g(y, z))`
```

## スクリプト

Ergのコードはスクリプトと呼ばれます。スクリプトはファイル形式(.er)で保存・実行できます。

## コメント

`#`以降はコメントとして無視されます。コードの意図を説明したいときや一時的にコードを無効化したいときなどに使います。

```python
# コメント
## `#`以降は改行されるまで無視されるので、`#`は何個あってもOK
#[
複数行コメント
対応する`]#`のところまでずっとコメントとして扱われます
]#
```

## 式、セパレータ

スクリプトは、式(expression)の連なりです。式とは計算・評価ができるもので、Ergではほとんどすべてのものが式です。
各式はセパレータ―改行かセミコロン`;`―で区切ります。
Ergのスクリプトは基本的に左から右へ、上から下へ評価されます。

```python
n = 1 # 代入式
f(1, 2) # 関数適用式
1 + 1 # 演算子適用式
f(1, 2); 1 + 1
```

以下のように、ブロック内で最後に評価した式を変数の値とするインスタントブロックという機能があります。
これは引数なし関数とは違い、`()`をつけません。ブロックがその場で1度だけ評価されることに注意してください。

```python
i =
    x = 1
    x + 1
assert i == 2
```

これはセミコロン(`;`)では実現できません。

```python
i = (x = 1; x + 1) # SyntaxError: cannot use `;` in parentheses
```

## インデント

ErgはPythonと同じくインデントを使ってブロックを表します。ブロックの開始を示すトリガーとなる演算子(特殊形式)は、`=`, `->`, `=>`, `do`, `do!`の5種類です(その他に、演算子ではありませんが`:`と`|`もインデントを生成します)。それぞれの意味は後述します。

```python
f x, y =
    x + y

for! 0..9, i =>
    print! i

for! 0..9, i =>
    print! i; print! i

ans = match x:
    0 -> "zero"
    _: 0..9 -> "1 dight"
    _: 10..99 -> "2 dights"
    _ -> "unknown"
```

また1行が長くなりすぎる場合、`\`を使って途中で改行させることができます。

```python
# this does not means `x + y + z` but means `x; +y; +z`
x
+ y
+ z

# this means `x + y + z`
x \
+ y \
+ z
```

<p align='center'>
    Previous | <a href='./01_literal.md'>Next</a>
</p>
