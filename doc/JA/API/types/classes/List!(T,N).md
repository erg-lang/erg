# List! T: Type, N: Nat

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/List!(T,N).md%26commit_hash%3Dcee3820a85d8a9dbdd1e506e6adc59eab5e19da1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/List!(T,N).md&commit_hash=cee3820a85d8a9dbdd1e506e6adc59eab5e19da1)

コンパイル時に長さのわかる可変長配列。`[T; N]!`という糖衣構文もある。

## methods

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]

  中の要素をランダムに選んでコピーを返す。

* shuffle!(ref! self)

  中身をシャッフルする。

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic

  長さを検証する。
  長さが不正な場合は`panic!`する。

## Impl

* From Range Int
* From [T; N]
* Num
