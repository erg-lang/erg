# testサブコマンド

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/test.md%26commit_hash%3D14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/test.md&commit_hash=14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)

ergコマンドにはtestというサブコマンドがあり、テスト実装、及び実行の支援を行う。

## Testデコレータ(@Test)

Ergではパッケージ中の`tests`ディレクトリか`*.test.er`ファイル中の`@Test`を付けたサブルーチンを`erg test`コマンドでテストする。
`tests`のサブルーチンはブラックボックステスト(非公開関数をテストしない)、`*.test.er`のサブルーチンはホワイトボックステスト(非公開関数もテストする)を担当する。

```python
# tests/test1.er
{add;} = import "foo"

@Test
test_1_plus_n(n: Nat) =
    assert add(1, n) == n + 1
```

実行結果がサマリとして表示され、各種ファイル形式(.md, .csv, etc.)で出力もできる。

## Doc Test

Ergでは`#`, `#[`以降がコメント行となるが、`##`, `#[[`でdoc commentとなり、VSCodeなどエディタからコメントをマークダウンで表示できる。
さらにdoc comment中のソースコードはergと指定されていれば、erg testコマンドで自動テストされる。
以下はテストの例である。

```python
VM = ...
    ...
    #[[
    execute commands.
    ```erg
    # 標準構成のVM
    {vm1;} = import "tests/mock"

    assert vm1.exec!("i = 0") == None
    assert vm1.exec!("i").try_into(Int)? == 0
    ```
    ]]#
    .exec! ref self, src =
        ...
    ...
```

テストの際に使う模擬オブジェクト(モックオブジェクト)は`tests/mock`モジュールに定義する。
