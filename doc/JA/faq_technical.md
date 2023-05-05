# 技術的なFAQ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/faq_technical.md%26commit_hash%3D1b3d7827bb770459475e4102c6f5c43d8ad79ae4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/faq_technical.md&commit_hash=1b3d7827bb770459475e4102c6f5c43d8ad79ae4)

本項はErg言語を使用する上での技術的な質問に答えるものです。すなわち、WhatやWhichで始まる質問、Yes/Noで答えられる質問を載せています。

根本的な文法の決定経緯については[こちら](./faq_syntax.md)を、なぜこの言語を作ったのか、この機能はどのように実装されているのかなど、より大きな話題は[こちら](./faq_general.md)を参照してください。

## Ergに例外機構はないのですか?

A: ありません。Ergでは代わりに`Result`型を使います。なぜErgに例外機構がないのかは[こちら](./faq_syntax.md#なぜergには例外機構がないのですか)を参照してください。

## ErgにはTypeScriptのAnyに相当する型はないのですか?

A: ありません。すべてのオブジェクトは少なくとも`Object`クラスに属しますが、この型は最小限の属性を提供するのみの型で、Anyのように好き放題はできません。
`Object`クラスは`match`などによる動的検査を通し目的の型に変換して使用します。Javaなどの`Object`と同じ類です。
Ergの世界では、TypeScriptのようにAPIの定義を辿ったらAnyだったという絶望・混沌は生まれないのです。

## Never, {}, None, (), NotImplemented, Ellipsisは何が違うのですか?

A: `Never`は「起こりえない」型です。実行時エラーを出すサブルーチンが、`Never`(または`Never`の合併型)を戻り値型とします。これを検知するとプログラムはすぐさま停止します。`Never`型は定義上すべての型のサブクラスでもありますが、`Never`型オブジェクトは決してErgコード上に出現しませんし、生成もされません。`{}`は`Never`と等価です。
`Ellipsis`は省略を表すオブジェクトで、Python由来です。
`NotImplemented`もPython由来です。これは未実装を表すマーカーとして使われますが、Ergではエラーを出す`todo`関数の方を推奨します。
`None`は`NoneType`のインスタンスです。`Option`型でよく使われます。
`()`はユニット型であり、そのインスタンス自身でもあります。これはプロシージャの戻り値など「意味のない値」を返したいとき使われます。

## なぜ`x = p!()`は有効なのに`f() = p!()`はEffectErrorとなるのですか?

A: `!`は副作用の産物につけるマーカーではなく、副作用を起こしうるオブジェクトに付けるマーカーだからです。
プロシージャ`p!`や可変型`T!`は副作用を起こす可能性がありますが、例えば`p!()`の戻り値が`Int`型だった場合、それ自体はもう副作用を起こしません。

## PythonのAPIを使用しようとしたとき、Pythonでは有効だったコードがErgでは型エラーになりました。これはどういうことですか?

A: ErgのAPIはなるべくPythonのAPIの仕様に忠実に型付けられていますが、どうしても表現しきれないケースもあります。
また、仕様上有効でも望ましくないと判断した入力(例えば、intを入力すべきところでfloatを入力してもよい仕様など)は、Erg開発チームの判断により型エラーとする可能性があります。

## Tupleにはなぜコンストラクタ(`__call__`)がないのですか?

Ergのタプルは長さがコンパイル時に決まっている必要があります。そのため、タプルを構築する手段はほぼリテラルのみです。
長さが実行まで不定の場合、代わりに不変配列(`Array`)を使うことになります。Ergの不変配列はPythonのタプルとほぼ同じです。

```erg
arr = Array map(int, input!().split " ")
```

## Pythonでは発生しなかった実行時エラーがErgでは発生しました。原因として何が考えられますか?

素朴に実装するとエラーとなる例としては以下のスクリプトがあります。

```erg
{main!; TestCase!} = pyimport "unittest"

Test! = Inherit TestCase!
Test!.
    test_one self =
        self.assertEqual 1, 1

main!()
```

基本的なunittestの使い方そのままであり、一見正しく見えますが、実行すると以下のようなエラーが出ます。

```console
AttributeError: 'Test!' object has no attribute '_testMethodName'
```

エラーが発生した原因は、TestCaseの実行の仕組みにあります。
TestCase(を継承したクラス)が実行されるとき、実行するテストメソッドは`test_`で始まる必要があります。
`test_one`はそれに従っているように見えますが、Ergは変数名に対して名前修飾(マングリング)を行います。
このせいでテストメソッドが認識されなくなっているのです。
マングリングを行わないようにするためには、''で囲む必要があります。

```erg
{main!; TestCase!} = pyimport "unittest"

Test! = Inherit TestCase!
Test!.
    'test_one' self =
        self.assertEqual 1, 1

main!()
```

今度は上手くいきます。

Erg特有のエラーが発生する場合は、名前修飾の影響などを疑ってみると良いでしょう。
