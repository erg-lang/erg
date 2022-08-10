# エラーハンドリングシステム

主にResult型を使用する。
ErgではError型オブジェクトを捨てる(トップレベルで対応しない)とエラーが発生する。

## 例外、Pythonとの相互運用

Ergは例外機構(Exception)を持たない。Pythonの関数をインポートする際は

* 戻り値を`T or Error`型とする
* `T or Panic`型(実行時エラーを出す可能性がある)とする

の2つの選択肢があり、`pyimport`ではデフォルトで後者となる。前者としてインポートしたい場合は、
`pyimport`の`exception_type`で`Error`を指定する(`exception_type: {Error, Panic}`)。

## 例外とResult型

`Result`型はエラーかもしれない値を表現する。`Result`によるエラーハンドリングはいくつかの点で例外機構よりも優れている。
まず第一に、サブルーチンがエラーを出すかもしれないと型定義から分かり、実際に使用するときも一目瞭然なのである。

```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

上の例では、例外がどの関数から送出されたものなのか、このコードだけでは分からない。関数定義まで遡っても、その関数が例外を出すかは判別しにくい。

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

翻って、こちらの例では`foo!`と`qux!`がエラーを出しうるとわかる。
正確には`y`も`Result`型である可能性があるが、中の値を使用するためにはいずれ対処しなくてはならない。

`Result`型を使用するメリットはそれだけではない。`Result`型はスレッドセーフでもある。これは、エラー情報を並列実行中に(容易に)受け渡しできるということを意味する。

## Context

`Error`/`Result`型単体では副作用が発生しないので、例外と違い送出場所などの情報(Context、文脈)を持てないが、`.context`メソッドを使えば`Error`オブジェクトに情報を付加できる。`.context`メソッドは`Error`オブジェクト自身を消費して新しい`Error`オブジェクトを作るタイプのメソッドである。チェイン可能であり、複数のコンテクストを保持できる。

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

なお、`.msg`や`.kind`などの`Error`の属性は副次的なものではないのでcontextではなく、最初に生成されたときのまま上書きできない。

## スタックトレース

`Result`型はその利便性から他言語でも多く取り入れられているが、例外機構と比較してエラーの発生元がわかりにくくなるというデメリットがある。
そこで、Ergでは`Error`オブジェクトに`.stack`という属性を持たせており、擬似的に例外機構のようなスタックトレースを再現している。
`.stack`は呼び出し元オブジェクトの配列である。Errorオブジェクトは`return`(`?`によるものも含む)されるたびにその呼出元サブルーチンを`.stack`に積んでいく。
そして`return`ができないコンテクストで`?`されるなり`.unwrap`されるなりすると、トレースバックを表示しながらパニックする。

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

Ergには回復不能なエラーへの対処として __パニッキング__ という機構も存在する。
回復不能なエラーとは、例えばソフト/ハードウェアの不具合など外的要因によるエラーや、それ以上コードを実行し続けても意味がないほど致命的なエラー、あるいはプログラム作成者の想定だにしないエラーなどである。これが発生した場合、プログラマの努力によって正常系に復帰させることができないため、その場でプログラムを終了させる。これを「パニックさせる」という。

パニックは`panic`関数で行う。

```erg
panic "something went wrong!"
```

<p align='center'>
    <a href='./29_decorator.md'>Previous</a> | <a href='./31_pipeline.md'>Next</a>
</p>
