# 版本控制

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/version.md%26commit_hash%3Dc1f43472c254e4c22f936b0f9157fc2ee3189697)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/version.md&commit_hash=c1f43472c254e4c22f936b0f9157fc2ee3189697)

Erg 编译器根据语义版本控制分配版本号。
但是，在版本 0 期间，应用的规则与平时不同（遵循比语义版本控制更详细的规则）。
需要注意的是，Erg 中有两种类型的兼容性。一个是规范兼容性，表示与语言规范的兼容性，另一个是内部兼容性，表示与（公共）API（如编译器）的兼容性。

* 在版本 0 期间，次要版本中的规范和内部兼容性可能会中断。这与正常的语义版本控制相同。
* 补丁版本不会破坏规范兼容性，但不能保证内部兼容性。
* 新功能主要在次要版本中添加，但如果它们是简单的语言功能或编译器功能，也可以在补丁版本中添加。

## 发布周期

* 补丁大约每 1~2 周发布一次。
* 次要版本的发布频率大约是补丁发布的 10 倍，即每 3~6 个月发布一次。
* 主要版本是无限期发布的。目前未计划版本 1 版本的计划。

## 每晚/测试版

Erg 将不定期进行夜间和测试版发布。每晚发布是新补丁版本的预发布，测试版是新的次要/主要版本的预发布。
每晚和测试版发布在 crates.io 上，测试版也发布在 GitHub 版本上。

每晚版本的格式是`0.x.y-nightly.z`。测试版也是如此。

几乎每天都会发布每晚版本（如果没有更改，则不会发布），而测试版则不定期发布。但是，一旦发布测试版，几乎每天都会发布新的测试版。