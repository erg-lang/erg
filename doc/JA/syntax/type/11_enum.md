# Enumerative Type(列挙型)

列挙型(Enum type)はSetによって生成されます。
列挙型はそのままでも型指定で使えるが、クラス化したりパッチを定義したりしてメソッド定義もできます。
列挙型による部分型システムを列挙的部分型付けといいます。

```erg
Bool = {True, False}
Status = {"ok", "error"}
```

`1..12`は`{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12}`と書き換えられるので、要素が有限の場合は本質的に列挙型と区間型は等価です。

```erg
Binary! = Class {0, 1}!.
    invert! ref! self =
        if! self == 0:
            do!
                self.set! 1
            do!
                self.set! 0

b = Binary!.new !0
b.invert!()
```

因みに、Ergの列挙型は他言語でよくある列挙型を包摂する概念です。

```rust
// Rust
enum Status { Ok, Error }
```

```erg
# Erg
Status = {"Ok", "Error"}
```

Rustとの相違点は、構造的部分型を採用しているというところにあります。

```rust
// StatusとExtraStatusの間にはなんの関係もない
enum Status { Ok, Error }
enum ExtraStatus { Ok, Error, Unknown }

// メソッドを実装可能
impl Status {
    // ...
}
impl ExtraStatus {
    // ...
}
```

```erg
# Status > ExtraStatusであり、Statusの要素はExtraStatusのメソッドを使える
Status = Trait {"Ok", "Error"}
    # ...
ExtraStatus = Trait {"Ok", "Error", "Unknown"}
    # ...
```

patchingによってメソッドの追加もできます。

明示的に包含関係を示したい場合、または既存のEnum型に選択肢を追加したい場合は`or`演算子を使います。

```erg
ExtraStatus = Status or {"Unknown"}
```

要素の属するクラスがすべて同一である列挙型を等質(homogenous)な列挙型といいます。
デフォルトでは、等質な列挙型を要件型とするクラスは、要素が属しているクラスのサブクラスとして扱えます。
あえてそうしたくない場合は、ラッパークラスとするとよいでしょう。

```erg
Abc = Class {"A", "B", "C"}
Abc.new("A").is_uppercase()

OpaqueAbc = Class {inner = {"A", "B", "C"}}.
    new inner: {"A", "B", "C"} = Self.new {inner;}
OpaqueAbc.new("A").is_uppercase() # TypeError
```
