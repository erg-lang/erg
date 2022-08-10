# Array! T

可変長配列を表す型。コンパイル時に長さがわからない場合に使う。`[T]!`という糖衣構文がある。
`Array! T = ArrayWithMutLength! T, !_`で定義される。
