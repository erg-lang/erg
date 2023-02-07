# Array! T, N

Variable length array at compile time. `[t; n]` There is also a sugar cane syntax.

## methods

* push! ref! self(N ~> N+1, ...), elem: T

* pop! ref! (N ~> N-1, ...) -> T

* sample!(ref! self) -> T
* sample! ref! self, M: Nat -> [T; M]


  Select a random element and return a copy.

* shuffle!(ref! self)


  Shuffle contents.

* assert_len ref! self(_ ~> N, ...), N: Nat -> () or Panic


  Verify length
  Incorrect length will cause `panic`

## Impl

* From Range Int
* From [T; N]
* Num
