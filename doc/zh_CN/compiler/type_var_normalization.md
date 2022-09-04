# Normalization

* Erg's type argument normalization is based on SymPy's simplify function.

For example, when you define `concat: |T, M, N|([T; M], [T; N] -> [T; M+N])`, you can match type variables and arguments without instantiating them. Judgment must be made.
Equality judgment naturally has its limits, but the judgments that are possible at present and their 方法 are as follows.

* Addition/multiplication symmetry:

  `n+m == m+n`

  Type variables are sorted and normalized as strings.

* Equivalence of addition and multiplication, subtraction and division:

  `n+n == 2*n`

  Normalize to Σ[c] x == c*x, where c is a constant.
  Constants are normalized by placing them on the left side of binary operations.

* Equality of double expressions:

  `n+m+l == m+n+l == l+m+n == ...`
  `n+m*l == m*l+n`

  Determined by normalizing by sorting.
  Blocks for multiplication and division are placed to the left of addition and subtraction. Blocks are sorted by comparing the type variables on the leftmost side.

* Basic inequalities:

  `n > m -> m + 1 > n`

* Equality:

  `n >= m and m >= n -> m == n`

* Transitivity of inequalities:

  `n > 0 -> n > -1`