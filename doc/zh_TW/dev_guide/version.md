# 版本控製

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/version.md%26commit_hash%3Dc1f43472c254e4c22f936b0f9157fc2ee3189697)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/version.md&commit_hash=c1f43472c254e4c22f936b0f9157fc2ee3189697)

Erg 編譯器根據語義版本控製分配版本號。
但是，在版本 0 期間，應用的規則與平時不同（遵循比語義版本控製更詳細的規則）。
需要註意的是，Erg 中有兩種類型的兼容性。一個是規範兼容性，表示與語言規範的兼容性，另一個是內部兼容性，表示與（公共）API（如編譯器）的兼容性。

* 在版本 0 期間，次要版本中的規範和內部兼容性可能會中斷。這與正常的語義版本控製相同。
* 補丁版本不會破壞規範兼容性，但不能保證內部兼容性。
* 新功能主要在次要版本中添加，但如果它們是簡單的語言功能或編譯器功能，也可以在補丁版本中添加。

## 發布周期

* 補丁大約每 1~2 周發布一次。
* 次要版本的發布頻率大約是補丁發布的 10 倍，即每 3~6 個月發布一次。
* 主要版本是無限期發布的。目前未計劃版本 1 版本的計劃。

## 每晚/測試版

Erg 將不定期進行夜間和測試版發布。每晚發布是新補丁版本的預發布，測試版是新的次要/主要版本的預發布。
每晚和測試版發布在 crates.io 上，測試版也發布在 GitHub 版本上。

每晚版本的格式是`0.x.y-nightly.z`。測試版也是如此。

幾乎每天都會發布每晚版本（如果沒有更改，則不會發布），而測試版則不定期發布。但是，一旦發布測試版，幾乎每天都會發布新的測試版。