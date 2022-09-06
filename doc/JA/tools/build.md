# buildサブコマンド

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/build.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/build.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

buildサブコマンドでは、パッケージのビルドを行います。
デフォルトのビルドで行われる工程は、以下の通りです。

1. コメント/ドキュメント(doc以下のmdファイル)内のコードを検査する。
2. パッケージに必要なコードをコンパイルする。
3. アプリケーションパッケージの場合は、コマンド相当のバッチファイルまたはシェルスクリプトを生成する。
4. テストを実行する。

ビルド終了後の成果物は以下のディレクトリに出力されます。

* デバッグビルド時: build/debug
* リリースビルド時: build/release
