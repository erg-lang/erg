# installサブコマンド

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/install.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/install.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

installでレジストリサイトに登録されたパッケージをインストールできる。
基本的な使い方はcargoなどのパッケージマネージャと同じ。

## 便利機能

* 似た名前のパッケージ名があり、そちらのほうが10倍以上ダウンロード数が多かった場合、間違えて入力したのではないかというサジェスチョンが出る。これにより、typo squattingを防止できる。
* パッケージサイズが大きい場合(50MB以上)、サイズを表示して本当にインストールするかサジェスチョンする。
* パッケージがduplicatedになっていた場合、代替のパッケージをサジェスチョンする。
