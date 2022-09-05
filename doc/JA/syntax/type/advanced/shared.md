# 共有参照(Shared Reference)


共有参照は気をつけて扱わねばならない言語機能の一つです。
例えばTypeScriptでは以下のようなコードが型検査を通ってしまいます。

```typescript
class NormalMember {}
class VIPMember extends NormalMember {}

let vip_area: VIPMember[] = []
let normal_area: NormalMember[] = vip_area

normal_area.push(new NormalMember())
console.log(vip_area) # [NormalMember]
```

一般会員がVIPエリアに侵入してしまっています。これは明らかなバグですが、何がいけなかったのでしょうか。
原因は共有参照の[変性](./variance.md)です。`normal_area`は`vip_area`をコピーして作成されていますが、その際に型が変わってしまっています。
しかし`VIPMember`は`NormalMember`を継承しているので`VIPMember[] <: NormalMember[]`となり、これは問題ないとされてしまっているのです。
`VIPMember[] <: NormalMember[]`という関係は、不変オブジェクトの場合は問題ありません。しかし上のように破壊的な操作を行ってしまうと、綻びが発生します。

Ergでは、所有権システムのおかげでこのようなコードは弾かれます。

```python
NormalMember = Class()
VIPMember = Class()

vip_area = [].into [VIPMember; !_]
normal_area: [NormalMember; !_] = vip_area

normal_area.push!(NormalMember.new())
log vip_area # OwnershipError: `vip_room` was moved to `normal_room`
```

しかし、オブジェクトの所有権が一箇所にしかない状態は不便である場合もあります。
そのためにErgは`SharedCell! T!`という型があり、これが共有状態を表します。

```python
$p1 = SharedCell!.new(!1)
$p2 = $p1.mirror!()
$p3 = SharedCell!.new(!1)
# $p1 == $p2とすると、中身の型Int!の比較が行われる
assert $p1 == $p2
assert $p1 == $p3
# $p1と$p2が同じものを指しているかは、`.addr!`で確認する
assert $p1.addr!() == $p2.addr!()
assert $p1.addr!() != $p3.addr!()
$p1.add! 1
assert $p1 == 2
assert $p2 == 2
assert $p3 == 1
```

`SharedCell!`型のオブジェクトは先頭に`$`を付ける必要があります。また、その性質上、定数にすることはできません。

`SharedCell! T!`型は`T!`型のサブタイプでもあり、`T!`型のメソッドを呼び出すことができます。`SharedCell! T!`型固有のメソッドは`.addr!`と`.mirror!`、`.try_take`のみです。

重要な事実として、`SharedCell! T!`は非変(non-variant)です。すなわち、型引数の違いによる包含関係が定義されません。

```python
$vip_area = SharedCell!.new([].into [VIPMember; !_])
$normal_area: SharedCell!([NormalMember; !_]) = $vip_area.mirror!() # TypeError: expected SharedCell!([NormalMember; !_]), but got SharedCell!([VIPMember; !_])
# hint: SharedCell!(T) is non-variant, which means it cannot have a supertype or a subtype.
```

しかし、以下のコードは問題ありません。最後の行では、型変換されたのは引数の`VIPMember`の方です。

```python
$normal_area = SharedCell!.new([].into [NormalMember; !_])
$normal_area.push!(NormalMember.new()) # OK
$normal_area.push!(VIPMember.new()) # OK
```
