# Array! T: Type, N: Nat

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Array!(T,N).md%26commit_hash%3Dcee3820a85d8a9dbdd1e506e6adc59eab5e19da1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Array!(T,N).md&commit_hash=cee3820a85d8a9dbdd1e506e6adc59eab5e19da1)

在编译时知道长度的可变长度数组。`[T; N]!`也有语法糖。

## methods

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]

  随机选择里面的元素并返回副本

* shuffle!(ref! self)

  打乱内容。

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic

  验证长度。
  长度不正确时会导致`panic`

## Impl

* From Range Int
* From [T; N]
* Num
