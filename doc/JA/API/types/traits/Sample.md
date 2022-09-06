# Sample

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Sample.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Sample.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

インスタンスを「適当に」選出する`sample`メソッドと`sample!`メソッドを持つトレイト。`sample`メソッドは常に同じインスタンスを返し、`sample!`メソッドは呼び出しごとに変わる適当なインスタンスを返す。

これはテストなどで適当なインスタンスがほしい場合を想定したトレイトであり、必ずしも無作為ではないことに注意が必要である。無作為抽出が必要な場合は`random`モジュールを使用する。

主要な値クラスは全て`Sample`を実装する。また、`Sample`なクラスで構成されているタプル型やレコード型、Or型、篩型でも実装されている。

```python
assert Int.sample() == 42
assert Str.sample() == "example"
# Intの場合、標準では64bitの範囲でサンプルされる
print! Int.sample!() # 1313798
print! {x = Int; y = Int}.sample!() # {x = -32432892, y = 78458576891}
```

以下は`Sample`の実装例である。

```python
EmailAddress = Class {header = Str; domain = Str}, Impl=Sample and Show
@Impl Show
EmailAddress.
    show self = "{self::header}@{self::domain}"
@Impl Sample
EmailAddress.
    sample(): Self = Self.new "sample@gmail.com"
    sample!(): Self =
        domain = ["gmail.com", "icloud.com", "yahoo.com", "outlook.com", ...].sample!()
        header = AsciiStr.sample!()
        Self.new {header; domain}
```
