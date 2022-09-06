# 正規化

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/type_var_normalization.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/type_var_normalization.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

* Ergの型引数正規化はSymPyのsimplify関数を参考にしています。

例えば`concat: |T, M, N|([T; M], [T; N] -> [T; M+N])`を定義するとき、型変数・引数を具体化せずに一致判定を行わなくてはならない。
等式判定は自ずと限界があるが、現時点で可能な判定とその方式は以下の通り。

* 加算・乗算の対称性：

  `n+m == m+n`

  型変数は文字列としてソートし正規化する。

* 加算と乗算、減算と除算の等価性:

  `n+n == 2*n`

  Σ[c] x == c*xに正規化する(cは定数)。
  定数は二項演算の左辺に置いて正規化する。

* 複式の等価性：

  `n+m+l == m+n+l == l+m+n == ...`
  `n+m*l == m*l+n`

  ソートで正規化して判定する。
  乗算・除算のブロックは加算・減算より左側に出す。ブロック同士は最左辺の型変数を比較してソートする。

* 基本的な不等式：

  `n > m -> m + 1 > n`

* 等式：

  `n >= m and m >= n -> m == n`

* 不等式の推移性：

  `n > 0 -> n > -1`
