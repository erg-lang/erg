# ArrayWithMutLength! T: Type, N: Nat&excl;

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/ArrayWithMutLength!(T,N).md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/ArrayWithMutLength!(T,N).md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

一个可变长度数组，其长度在编译时已知。还有语法糖`ArrayWithMutLength(T, !N) == [T; !N]`

## 方法

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]
  随机选择里面的一个元素并返回一个副本

* shuffle!(ref! self)
  把里面的东西随机摆放

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic
  验证长度
  `panic!` 如果长度无效

## Impl

* From Range Int
* From [T; N]
* Num
