# Into T

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Into.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Into.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

T型に型変換可能であることを示す型です。
SelfとTの間に継承関係などがなくても、互いに変換可能な関係であるときに定義します。
継承と違い暗黙には変換が行われません。必ず`.into`メソッドを呼び出す必要があります。

## methods

* into(self, T) -> T

  変換を行います。
