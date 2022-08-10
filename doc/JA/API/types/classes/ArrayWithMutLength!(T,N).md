# ArrayWithMutLength! T: Type, N: Nat&excl;

コンパイル時に長さのわかる可変長配列。`ArrayWithMutLength(T, !N) == [T; !N]`という糖衣構文もある。

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
