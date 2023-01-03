# 基本事項

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/00_basic.md%26commit_hash%3Dd0a980e7401b15c8d2adc870e42952b8552973f1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/00_basic.md&commit_hash=d0a980e7401b15c8d2adc870e42952b8552973f1)

> __Warning__: 本ドキュメントは未完成です。校正(文体、正しいリンクが張られているか、など)がなされていません。また、Ergの文法はバージョン0.*の間に破壊的変更が加えられる可能性があり、それに伴うドキュメントの更新が追いついていない可能性があります。予めご了承ください。
> また、本ドキュメントの誤りを見つけた場合は、[こちらのフォーム](https://forms.gle/HtLYRfYzWCAaeTGb6)または[GitHubリポジトリ](https://github.com/erg-lang/erg/issues/new?assignees=&labels=bug&template=bug_report.yaml)から修正の提案をしていただけると幸いです。

本ドキュメントは、Ergの基本文法について解説するものです。
既にpythonなどの言語に触れた経験がある方は、概説的な[quick&nbsp;tour](./quick_tour.md)もあるためそちらを参照してください。
また、[標準API](../API/index.md)や[Ergコントリビューター向けの内部資料](../dev_guide/index.md)は別途存在します。文法やErg本体についての詳細な説明が必要な場合はそちらを参照してください。

## Hello, World&excl;

まずは恒例の、Hello Worldを行いましょう。

```python
print!("Hello, World!")
```

Pythonや同系統の言語とほぼ同じです。目を引くのは`print`の後に付く`!`ですが、これの意味はおいおい説明します。
また、Ergでは解釈に紛れのない限り括弧`()`を省略することが出来ます。
括弧の省略ができるのはRubyと似ていますが、複数の解釈ができる括弧省略はできませんし、また引数が0個のときもPythonと同じく`()`の省略が出来ません。

```python.checker_ignore
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OKだが呼び出しという意味ではなく、単に`print!`を呼び出し可能なオブジェクトとして取得するという意味となる

print! f x # OK、これは`print!(f(x))`として解釈される
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, `print!(f(x), y)`または`print!(f(x, y))`の二通りの解釈ができてしまう
print!(f x, y) # NG, `print!(f(x), y)`または`print!(f(x, y))`の二通りの解釈ができてしまう
print! f(x, g y, z) # NG, `print!(x, g(y), z)`または`print!(x, g(y, z))`の二通りの解釈ができてしまう
```

## スクリプト

Ergのコードはスクリプトと呼ばれます。スクリプトはファイル形式(.er)で保存・実行できます。

以下のコードを"hello.er"として保存します。

```python
print! "hello, world"
```

ターミナルですぐに実行することができます。

```sh
$ erg hello.er
hello, world
```

## コメント

`#`以降はコメントとして無視されます。コードの意図を説明したいときや一時的にコードを無効化したいときなどに使います。

```python
# コメント
## `#`以降は改行されるまで無視されるため、`#`は何個でも使用できる
#[
複数行コメント
対応する`#[`から`]#`のところまでがコメントとして扱われる
]#
```

## 式、セパレータ

スクリプトは、式(expression)の連なりです。式とは計算・評価ができるもので、Ergではほとんどすべてのものが式です。
各式はセパレータ―改行かセミコロン`;`―で区切ります。
Ergのスクリプトは基本的に左から右へ、上から下へ評価されます。

```python
n = 1 # 代入式
f x, y = x + y # 関数定義
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

```python,compile_fail
i = (x = 1; x + 1) # SyntaxError:  cannot use `;` in parentheses
```

## インデント

ErgはPythonと同じくインデントを使ってブロックを表します。ブロックの開始を示すトリガーとなる演算子(特殊形式)は、`=`, `->`, `=>`の3種類です(その他に、演算子ではありませんが`:`と`|`もインデントを生成します)。それぞれの意味は後述します。

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
# これは`x + y + z`と解釈されず`x; +y; +z`と解釈される
x
+ y
+ z

# これは`x + y + z`と解釈される
x \
+ y \
+ z
```

<p align='center'>
    Previous | <a href='./01_literal.md'>Next</a>
</p>
