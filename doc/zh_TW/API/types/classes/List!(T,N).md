# List! T: Type, N: Nat

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/List!(T,N).md%26commit_hash%3Dcee3820a85d8a9dbdd1e506e6adc59eab5e19da1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/List!(T,N).md&commit_hash=cee3820a85d8a9dbdd1e506e6adc59eab5e19da1)

在編譯時知道長度的可變長度數組。`[T; N]!`也有語法糖。

## methods

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]

  隨機選擇裏面的元素並返回副本
* shuffle!(ref! self)

  打亂內容。

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic

  驗證長度。
  長度不正確時會導致`panic`

## Impl

* From Range Int
* From [T; N]
* Num
