# 型境界 (Type Bound)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/19_bound.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/19_bound.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

型境界は型指定に条件を加えるものである。これを実現する機能がガード(ガード節)である。
関数シグニチャ、無名関数シグニチャのほか、篩型でもこの機能を利用できる。
ガードは戻り値型の後に記述する。

## 述語式 (Predicate)

変数の満たす条件を、`Bool`を返す式(述語式)で指定できる。
使用できるのは[値オブジェクト](./08_value.md)と演算子だけである。コンパイル時関数は今後のバージョンで対応される可能性がある。

```python
f a: [T; N] | T, N, N > 5 = ...
g a: [T; N | N > 5] | T, N = ...
Odd = {I: Int | I % 2 == 1}
R2Plus = {(L, R) | L, R: Ratio; L > 0 and R > 0}
GeneralizedOdd = {I | U; I <: Div(Nat, U); I % 2 == 0}
```
