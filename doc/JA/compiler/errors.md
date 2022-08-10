# Erg Compiler Errors

## AssignError

イミュータブル変数を書き換えようとすると発生します。

## AttributeError

存在しない属性にアクセスしようとすると発生します。

## SideEffectError

副作用が許可されていないスコープ(関数、不変型など)で副作用を起こすコードを記述すると発生します。

## OwnershipError

既にムーブ済みの変数にアクセスしようとすると発生します。

## BorrowError

あるオブジェクトに対する借用が存在している間にもう一つ可変参照を取得しようとすると発生します。

## CyclicError

明らかに停止しない循環を起こしている場合に発生します。

```erg
i: Int = i

f(): Int = g()
g() = f()

h(): Int = module::h()

T = U
U = T
```

## BytecodeError

読み込んだバイトコードが破損していた場合に発生します。

## CompileSystemError

コンパイラ内部でエラーが起きた場合に発生します。

## EnvironmentError

インストール時にアクセス権限がなかった場合などで発生します。

## FeatureError

正式に提供されていない試験的機能を検出した際に発生します。

## ImportError

## IndentationError

不正なインデントを検出すると発生します。
SyntaxErrorの派生です。

## NameError

存在しない変数にアクセスすると発生します。

## NotImplementedError

定義は存在し、実装のないAPIを呼び出すと発生します。
TypeErrorの派生です。

## PatternError

不正なパターンを検出すると発生します。
SyntaxErrorの派生です。

## SyntaxError

不正な文法を検出すると発生します。

## TabError

インデント/スペースとしてタブ文字を使うと発生します。
SyntaxErrorの派生です。

## TypeError

オブジェクトの型が合わない際に発生します。

## UnboundLocalError

変数を定義前に使用すると発生します。
正確には、あるスコープ内で定義された変数がそれ以前に使われていると発生します。

```erg
i = 0
f x =
    y = i + x
    i = 1
    y + i
```

このコードでは`y = i + x`の`i`が未定義変数になります。
しかし、定数の場合は定義前に別の関数中で呼び出し可能です。

```erg
f() = g()
g() = f()
```

## Erg Compiler Warnings

## SyntaxWarning

文法上は問題ありませんが、冗長だったり一般的でないコードを検出した際に発生します(不要な`()`など)。

```erg
if (True): # SyntaxWarning: unnecessary parentheses
    ...
```

## DeprecationWarning

参照したオブジェクトが非推奨である場合に発生します。
(開発者はこのWarningを発生させる際、必ず代替手段をHintとして提示してください)

## FutureWarning

将来的に問題が起こりそうなコードを検出すると発生します。
このWarningはバージョンの互換性(ライブラリ含む)の問題や文法・APIの変更によって起こります。

## ImportWarning
