# 放棄・却下された言語仕様

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/abandoned.md%26commit_hash%3D00350f64a40b12f763a605bc16748d09379ab182)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/abandoned.md&commit_hash=00350f64a40b12f763a605bc16748d09379ab182)

## オーバーロード(アドホック多相)

パラメトリック+サブタイピング多相で代替できること、Pythonの意味論との相性の悪さなどを理由に放棄された。詳しくは[overload](../syntax/type/advanced/overloading.md)の記事を参照。

## 明示的ライフタイム付き所有権システム

Rustのような所有権システムを導入する予定だったが、Pythonの意味論との相性の悪さ、ライフタイム注釈など煩雑な仕様の導入が必要であることなどから放棄され、不変オブジェクトは全てRCで管理され、可変オブジェクトの所有権は1つのみ、という文法になった。
DyneはC# , NimのようにGILを持たず、値オブジェクトや安全な範囲での低レベル操作もできるようにする方針である。
