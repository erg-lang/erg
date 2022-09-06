# envサブコマンド

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/env.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/env.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

envサブコマンドはerg実行環境の指定を行います。
`erg env new [env name]`で新しい実行環境を作成します。対話ツールが開き、ergのバージョンを指定すると、そのバージョンのergがインストール(すでにあれば流用されます)され、新しい環境として使えるようになります。
`erg env switch [env name]`で環境の切り替えができます。
作成された環境は`erg env edit`で編集でき、パッケージをプリインストールしたり、他言語の依存関係を指定できる。
このコマンドの最大の特徴は`erg env export`で環境を再現する情報を`[env name].env.er`ファイルとして出力できる点である。これにより、他人と同じ環境ですぐに開発を始められる。さらに`erg env publish`でパッケージのように環境を公開できる。
