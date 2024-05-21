# module `unsound`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/unsound.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/unsound.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Ergの型システムでは安全が保証できない、不安全で不健全な操作を行うAPIを提供します。

## `unsafe!`

Unsafe
Executes a `Unsafe` procedure. Just like Rust, `Unsafe` APIs cannot be called directly, but are all passed as higher-order functions to this procedure.

```python
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
