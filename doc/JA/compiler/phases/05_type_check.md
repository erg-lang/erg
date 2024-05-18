# Type checking (型解析)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/05_type_check.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/05_type_check.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

Ergの型解析は、フェーズとしてはASTをHIRに変換するlowering(低位化)フェーズの一部である。HIRはASTよりも若干コンパクトな構文木(中間表現)であり、全ての式に対し型が明示されている。
loweringを行うのは`ASTLowerer`であり、型解析を実行するのは`Context`という構造体である。

Ergの型解析は型検査と型推論の２つの側面を持つ。両者は同時に実行される。
型検査では、型指定と型環境を参照して項が規則通りに使用されているか検査する。型推論では、型指定がなされていない箇所に対して型変数を発行し、型環境を参照して単一化する。
