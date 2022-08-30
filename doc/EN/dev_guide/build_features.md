# `erg` build features

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/build_features.md%26commit_hash%3Dbd59088c51941b5336e2115189579171d8086929)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/build_features.md&commit_hash=bd59088c51941b5336e2115189579171d8086929)

## debug

Put into debug mode. This will log the behavior of Erg internally as it happens.
Independent of Rust's `debug_assertions` flag.

## japanese

Set the system language to Japanese.
In this build, Erg internal options, help (help, copyright, license, etc.) and error messages are guaranteed to be in Japanese.
