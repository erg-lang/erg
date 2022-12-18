# `erg` build features

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/build_features.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/build_features.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## debug

デバッグモードにする。これにより、Erg内部での挙動が逐次ログ表示される。
Rustの`debug_assertions`フラグとは独立。

## japanese

システム言語を日本語にする。
Erg内部のオプション、ヘルプ(help, copyright, licenseなど)、エラー表示は日本語化が保証される。

## simplified_chinese

システム言語を簡体字中国語に設定します。
Erg 内部オプション、ヘルプ (ヘルプ、著作権、ライセンスなど)、エラーは簡体字中国語で表示されます。

## traditional_chinese

システム言語を繁体字中国語に設定します。
Erg 内部オプション、ヘルプ (ヘルプ、著作権、ライセンスなど)、エラーは繁体字中国語で表示されます。

## unicode/pretty

コンパイラが表示をリッチにする。

## pre-commit

pre-commitでテストを実行する為に使用される。バグワークアラウンドである。

## large_thread

スレッドのスタックサイズを大きくする。Windowsでの実行やテスト実行のために使用される。
