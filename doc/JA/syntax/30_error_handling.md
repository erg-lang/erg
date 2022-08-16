# エラーハンドリングシステム

主にResult型を使用します。
ErgではError型オブジェクトを捨てる(トップレベルで対応しない)とエラーが発生します。

## 例外、Pythonとの相互運用

Ergは例外機構(Exception)を持ちません。Pythonの関数をインポートする際は

* 戻り値を`T or Error`型とする
* `T or Panic`型(実行時エラーを出す可能性がある)とする

の2つの選択肢があり、`pyimport`ではデフォルトで後者となる。前者としてインポートしたい場合は、
`pyimport`の`exception_type`で`Error`を指定する(`exception_type: {Error, Panic}`)。

## 例外とResult型

`Result`型はエラーかもしれない値を表現します。`Result`によるエラーハンドリングはいくつかの点で例外機構よりも優れています。
まず第一に、サブルーチンがエラーを出すかもしれないと型定義から分かり、実際に使用するときも一目瞭然です。

```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

上の例では、例外がどの関数から送出されたものなのか、このコードだけでは分かりません。関数定義まで遡っても、その関数が例外を出すかを判別するのは難しいです。

```erg
# Erg
try!:
    do!:
        x = foo!()?.bar()
        y = baz!()
        qux!()?
    e =>
        print! e
```

翻って、こちらの例では`foo!`と`qux!`がエラーを出しうるとわかります。
正確には`y`も`Result`型である可能性がありますが、中の値を使用するためにはいずれ対処しなくてはなりません。

`Result`型を使用するメリットはそれだけではありません。`Result`型はスレッドセーフでもあります。これは、エラー情報を並列実行中に(容易に)受け渡しできるということを意味します。

## Context

`Error`/`Result`型単体では副作用が発生しないので、例外と違い送出場所などの情報(Context、文脈)を持てませんが、`.context`メソッドを使えば`Error`オブジェクトに情報を付加できます。`.context`メソッドは`Error`オブジェクト自身を消費して新しい`Error`オブジェクトを作るタイプのメソッドです。チェイン可能であり、複数のコンテクストを保持できます。

```erg
f() =
    todo() \
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

なお、`.msg`や`.kind`などの`Error`の属性は副次的なものではないのでcontextではなく、最初に生成されたときのまま上書きできません。

## スタックトレース

`Result`型はその利便性から他言語でも多く取り入れられていますが、例外機構と比較してエラーの発生元がわかりにくくなるというデメリットがあります。
そこで、Ergでは`Error`オブジェクトに`.stack`という属性を持たせており、擬似的に例外機構のようなスタックトレースを再現しています。
`.stack`は呼び出し元オブジェクトの配列です。Errorオブジェクトは`return`(`?`によるものも含む)されるたびにその呼出元サブルーチンを`.stack`に積んでいきます。
そして`return`ができないコンテクストで`?`されるなり`.unwrap`されるなりすると、トレースバックを表示しながらパニックします。

```erg
f x =
    ...
    y = foo.try_some(x)?
    ...

g x =
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
#    ...
#    Foo.try_some, line 10, file "foo.er"
#    10 | y = foo.try_some(x)?
#    module::f, line 23, file "foo.er"
#    23 | y = f(x)?
#    module::g, line 40, file "foo.er"
#    40 | i = g(1)?
# Error: ...
```

## パニック

Ergには回復不能なエラーへの対処として __パニッキング__ という機構も存在します。
回復不能なエラーとは、例えばソフト/ハードウェアの不具合など外的要因によるエラーや、それ以上コードを実行し続けても意味がないほど致命的なエラー、あるいはプログラム作成者の想定だにしないエラーなどです。これが発生した場合、プログラマの努力によって正常系に復帰させることができないため、その場でプログラムを終了させます。これを「パニックさせる」といいます。

パニックは`panic`関数で行います。

```erg
panic "something went wrong!"
```

<p align='center'>
    <a href='./29_decorator.md'>Previous</a> | <a href='./31_pipeline.md'>Next</a>
</p>
