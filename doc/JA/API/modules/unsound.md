# module `unsound`

Provides APIs perform unsound and unsafe operations that cannot be guaranteed safe in Erg's type system.

## `unsafe!`

Executes a `Unsafe` procedure. Just like Rust, `Unsafe` APIs cannot be called directly, but are all passed as higher-order functions to this procedure.

```erg
unsound = import "unsound"

i = unsound.unsafe! do!:
    # convert `Result Int` to `Int`
    unsound.transmute input!().try_into(Int), Int
```

## transmute

第1引数のオブジェクトを第2引数の型へ変換します。型チェックは行われません。
この関数は型システムの型安全性を損ないます。使用の際はバリデーション等を行ってください。

## auto_transmute

`transmute`とは違い、期待される型に自動で変換します。Ocamlの`Obj.magic`と同じ働きをします。
