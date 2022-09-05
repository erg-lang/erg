# ArrayWithMutLength! T: Type, N: Nat&excl;

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
  驗證長度。
  `panic!` 如果長度無效。

## Impl

* From Range Int
* From [T; N]
* Num
