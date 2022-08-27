# testサブコマンド

ergコマンドにはtestというサブコマンドがあり、テスト実装、及び実行の支援を行う。

## Testデコレータ(@Test)

Ergではパッケージ中の`tests`ディレクトリか`*.test.er`ファイル中の`@Test`を付けたサブルーチンを`erg test`コマンドでテストする。
`tests`のサブルーチンはブラックボックステスト(非公開関数をテストしない)、`*.test.er`のサブルーチンはホワイトボックステスト(非公開関数もテストする)を担当する。

```erg
# tests/test1.er
{add; ...} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

実行結果がサマリとして表示され、各種ファイル形式(.md, .csv, etc.)で出力もできる。

## Doc Test

Ergでは`#`, `#[`以降がコメント行となるが、`##`, `#[[`でdoc commentとなり、VSCodeなどエディタからコメントをマークダウンで表示できる。
さらにdoc comment中のソースコードはergと指定されていれば、erg testコマンドで自動テストされる。
以下はテストの例である。

```erg
VM = ...
    ...
    #[[
    execute commands.
    ```erg
    # VM in standard configuration
    {vm1; ...} = import "tests/mock"

    assert vm1.exec!("i = 0") == None
    assert vm1.exec!("i").try_into(Int)? == 0
    ```
    ]]#
    .exec! ref self, src =
        ...
    ...
```

テストの際に使う典型的なオブジェクト(モックオブジェクト)は`tests/mock`モジュールに定義する。
