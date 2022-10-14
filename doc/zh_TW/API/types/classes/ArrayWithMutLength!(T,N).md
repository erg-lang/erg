# ArrayWithMutLength! T: Type, N: Nat&excl;

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/ArrayWithMutLength!(T,N).md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/ArrayWithMutLength!(T,N).md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

一個可變長度數組，其長度在編譯時已知。還有語法糖`ArrayWithMutLength(T, !N) == [T; !N]`

## 方法

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]
  隨機選擇里面的一個元素并返回一個副本

* shuffle!(ref! self)
  把里面的東西隨機擺放

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic
  驗證長度
  `panic!` 如果長度無效

## Impl

* From Range Int
* From [T; N]
* Num
