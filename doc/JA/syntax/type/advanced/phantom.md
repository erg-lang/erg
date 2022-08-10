# 幽霊型(Phantom class)

幽霊型は、コンパイラに注釈を与えるためだけに存在するマーカートレイトである。
幽霊型の使い方として、リストの構成をみる。

```erg
Nil = Class()
List T, 0 = Inherit Nil
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

このコードはエラーとなる。

```erg
3 | List T, 0 = Inherit Nil
                        ^^^
TypeConstructionError: since Nil does not have a parameter T, it is not possible to construct List(T, 0) with Nil
hint: use 'Phantom' trait to consume T
```

このエラーはつまり、`List(_, 0).new Nil.new()`とされたときに`T`の型推論ができないという文句である。Ergでは型引数を未使用のままにすることができないのである。
このような場合は何でもよいので`T`型を右辺で消費する必要がある。サイズが0の型、例えば長さ0のタプルならば実行時のオーバーヘッドもなく都合がよい。

```erg
Nil T = Class((T; 0))
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

このコードはコンパイルを通る。だが少しトリッキーで意図が分かりづらい上に、型引数が型のとき以外では使えない。

このようなときにちょうどよいのが幽霊型である。幽霊型はサイズ0の型を一般化した型である。

```erg
Nil T = Class(Impl: Phantom T)
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}

nil = Nil(Int).new()
assert nil.__size__ == 0
```

`Phantom`が`T`型を保持する。しかし実際には`Phantom T`型のサイズは0であり、`T`型のオブジェクトを保持してはいない。

また、`Phantom`は型以外にも任意の型引数を消費することができる。以下の例では`State`という`Str`のサブタイプオブジェクトである型引数を`Phantom`が保持している。
この場合も、`state`はオブジェクトの実体に現れないハリボテの型変数である。

```erg
VM! State: {"stopped", "running"}! = Class(..., Impl: Phantom! State)
VM!("stopped").
    start ref! self("stopped" ~> "running") =
        self.do_something!()
        self::set_phantom!("running")
```

`state`は`update_phantom!`メソッドか`set_phantom!`メソッドを介して更新する。
これは`Phantom!`(`Phantom`の可変版)の標準パッチが提供するメソッドで、使い方は可変型の`update!`, `set!`と同じである。
