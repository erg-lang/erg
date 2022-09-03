# ArrayWithMutLength! T: Type, N: Nat&excl;

一个可变长度数组，其长度在编译时已知。还有语法糖`ArrayWithMutLength(T, !N) == [T; !N]`

## methods

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]
  随机选择里面的一个元素并返回一个副本

* shuffle!(ref! self)
  把里面的东西随机摆放

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic
  验证长度。
  `panic!` 如果长度无效。

## Impl

* From Range Int
* From [T; N]
* Num
