# Patch

Ergでは、既存の型・クラスに手を加えることはできません。
クラスにメソッドを追加で定義することはできず、特殊化(specialization, 多相に宣言された型を単相化し専用のメソッドを定義する機能。C++などが持つ)も行えません。
しかし、既存の型・クラスに機能を追加したいという状況は多々あり、これを実現するためにパッチという機能があります。

```python
StrReverse = Patch Str
StrReverse.
    reverse self = self.iter().rev().collect(Str)

assert "abc".reverse() == "cba"
```

パッチの名前は、主に追加する機能を端的に表すものがよいでしょう。
こうすると、パッチされる型(`Str`)のオブジェクトはパッチ(`StrReverse`)のメソッドを使えるようになります。
実際、`.reverse`は`Str`のメソッドではなく、`StrRReverse`に追加されたメソッドです。

ただし、パッチのメソッドは記名型(クラス)のメソッドより優先度が低く、既存のクラスのメソッドをオーバーライド(上書き)できません。

```python
StrangeInt = Patch Int
StrangeInt.
    `_+_` = Int.`_-_` # AssignError: .`_+_` is already defined in Int
```

オーバーライドしたければ、クラスを継承する必要があります。
ただし、基本的にはオーバーライドを行わず、別の名前のメソッドを定義することを推奨します。
オーバーライドは安全のためいくつかの制約が課されており、それほど気軽に行えるものではないからです。

```python
StrangeInt = Inherit Int
StrangeInt.
    # オーバーライドするメソッドにはOverrideデコレータを付与する必要がある
    # さらに、Int.`_+_`に依存するIntのメソッドすべてをオーバーライドする必要がある
    @Override
    `_+_` = Super.`_-_` # OverrideError: Int.`_+_` is referenced by ..., so these method must also be overridden
```

## パッチの選択

パッチは一つの型に対して複数定義し、まとめることもできます。

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
StrMultiReplace = Patch(Str)
StrMultiReverse.
    multi_replace self, pattern_and_targets: [(Pattern, Str)] = ...
StrToCamelCase = Patch(Str)
StrToCamelCase.
    to_camel_case self = ...
StrToKebabCase = Patch(Str)
StrToKebabCase.
    to_kebab_case self = ...

StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
```

```python
{StrBoosterPack; ...} = import "foo"

assert "abc".reverse() == "cba"
assert "abc".multi_replace([("a", "A"), ("b", "B")]) == "ABc"
assert "to camel case".to_camel_case() == "toCamelCase"
assert "to kebab case".to_kebab_case() == "to-kebab-case"
```

複数のパッチが定義できると、中には実装の重複が発生する可能性があります。

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
# more efficient implementation
StrReverseMk2 = Patch(Str)
StrReverseMk2.
    reverse self = ...

"hello".reverse() # PatchSelectionError: multiple choices of `.reverse`: StrReverse, StrReverseMk2
```

そのような場合は、メソッド形式ではなく関連関数形式とすることで一意化できます。

```python
assert StrReverseMk2.reverse("hello") == "olleh"
```

また、選択的にインポートすることでも一意化できます。

```python
{StrReverseMk2; ...} = import "foo"

assert StrReverseMk2.reverse("hello") == "olleh"
```

## 接着パッチ(Glue Patch)

パッチは型同士を関係付けることもできます。`StrReverse`は`Str`と`Reverse`を関係付けています。
このようなパッチは接着パッチ(Glue Patch)と呼ばれます。
`Str`は組み込みの型であるため、ユーザーがトレイトを後付けするためには接着パッチが必要なわけです。

```python
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch Str, Impl := Reverse
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

接着パッチは一つの型とトレイトのペアに対して一つまでしか定義できません。
これは、仮に複数の接着パッチが同時に「見える」場合、どの実装を選択するか一意に決められなくなるからです。
ただし、別のスコープ(モジュール)に移る際にパッチを入れ替えることはできます。

```python
NumericStr = Inherit Str
NumericStr.
    ...

NumStrRev = Patch NumericStr, Impl := Reverse
NumStrRev.
    ...
# DuplicatePatchError: NumericStr is already associated with `Reverse`
# hint: `Str` (superclass of `NumericStr`) is associated with `Reverse` by `StrReverse`
```

## Appendix: Rustのトレイトとの関連

ErgのパッチはRustの(後付けの)implブロックに相当します。

```rust
// Rust
trait Reverse {
    fn reverse(self) -> Self;
}

impl Reverse for String {
    fn reverse(self) -> Self {
        self.chars().rev().collect()
    }
}
```

RustのトレイトはErgのトレイトとパッチの機能を併せ持つ機能だと言えるでしょう。こう言うとRustのトレイトの方が便利に聞こえますが、実はそうとも限りません。

```python
# Erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch(Str, Impl := Reverse)
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

Ergではimplブロックがパッチとしてオブジェクト化されているため、他のモジュールから取り込む際に選択的な取り込みが可能になります。さらに副次的な効果として、外部構造体への外部トレイトの実装も可能となっています。
また、dyn traitやimpl traitといった文法も構造型によって必要なくなります。

```python
# Erg
reversible: [Reverse; 2] = [[1, 2, 3], "hello"]

iter|T|(i: Iterable T): Iterator T = i.iter()
```

```rust
// Rust
let reversible: [Box<dyn Reverse>; 2] = [Box::new([1, 2, 3]), Box::new("hello")];

fn iter<I>(i: I) -> impl Iterator<Item = I::Item> where I: IntoIterator {
    i.into_iter()
}
```

## 全称パッチ

パッチはある特定の型ひとつだけではなく、「関数の型全般」などに対しても定義できます。
この場合、自由度を与えたい項を引数にします(下の場合は`T: Type`)。このようにして定義したパッチを全称パッチといいます。
見れば分かる通り、全称パッチは正確にはパッチを返す関数ですが、それ自体もパッチとみなすことが可能です。

```python
FnType T: Type = Patch(T -> T)
FnType(T).
    type = T

assert (Int -> Int).type == Int
```

## 構造的パッチ

さらにパッチは、ある構造を満たす型すべてに定義することもできます。
ただしこれは記名的なパッチやクラスメソッドより優先度は低くなっています。

以下のように拡張によって成り立たなくなる性質もあるので、構造的パッチを定義する際は慎重に設計してください。

```python
# これはStructuralにするべきではない
Norm = Structural Patch {x = Int; y = Int}
Norm.
    norm self = self::x**2 + self::y**2

Point2D = Class {x = Int; y = Int}
assert Point2D.new({x = 1; y = 2}).norm() == 5

Point3D = Class {x = Int; y = Int; z = Int}
assert Point3D.new({x = 1; y = 2; z = 3}).norm() == 14 # AssertionError:
```

<p align='center'>
    <a href='./06_nst_vs_sst.md'>Previous</a> | <a href='./08_value.md'>Next</a>
</p>
