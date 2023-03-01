# 構造型

トレイトやクラスなどの型は構造化できます。こうすると、明示的に実装を宣言する必要がなくなります。Pythonにおけるダックタイピングを実現する機能と言えます。

```python
add|T, U| x: Structural { .__add__ = (self: T, other: U) -> V }, y: U = x.__add__ y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    # C|<: Add(C)|で明示的に実装したわけでないことに注意
    __add__ self, other: Self = Self.new {i = self::i + other::i}

assert add(C.new(1), C.new(2)) == C.new(3)
```

通常のトレイト、すなわち記名的トレイトは単に要求メソッドを実装しただけでは使えず、実装したことを明示的に宣言する必要があります。
以下の例では明示的な実装の宣言がないため、`add`が`C`型の引数で使えません。

```python,compile_fail
Add = Trait {
    .__add__ = (self: Self, other: Self) -> Self
}
# |A <: Add|は省略できる
add|A <: Add| x, y: A = x.__add__ y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    __add__ self, other: Self = Self.new {i = self::i + other::i}

add C.new(1), C.new(2) # TypeError: C is not subclass of Add
# hint: inherit or patch 'Add'
```

構造型はこの実装の宣言がなくてもよいのですが、そのかわり型推論が効かない場合があります。その際は型指定が必要です。
