# 归一化

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/type_var_normalization.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/type_var_normalization.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

* Erg 的类型参数规范化基于 SymPy 的简化函数。

例如，当您定义 `concat: |T, M, N|([T; M], [T; N] -> [T; M+N])` 时，您可以匹配类型变量和参数而无需实例化它们.必须作出判断。
平等判断自然有其局限性，但目前可能的判断及其方法如下。

* 加法/乘法对称性：

  `n+m == m+n`

  类型变量被排序和规范化为字符串。

* 加法、乘法、减法和除法等价：

  `n+n == 2*n`

  归一化为 Σ[c] x == c*x，其中 c 是一个常数。
  常量通过将它们放在二进制操作的左侧进行标准化。

* 双重表达式的相等性：

  `n+m+l == m+n+l == l+m+n == ...`
  `n+m*l == m*l+n`

  通过排序归一化确定。
  乘法和除法块放置在加法和减法的左侧。通过比较最左侧的类型变量对块进行排序。

* 基本不等式：

  `n > m -> m + 1 > n`

* 平等：

  `n >= m 和 m >= n -> m == n`

* 不等式的传递性：

  `n > 0 -> n > -1`