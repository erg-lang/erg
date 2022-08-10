# Sample

インスタンスを「適当に」選出する`sample`メソッドと`sample!`メソッドを持つトレイト。`sample`メソッドは常に同じインスタンスを返し、`sample!`メソッドは呼び出しごとに変わる適当なインスタンスを返す。

これはテストなどで適当なインスタンスがほしい場合を想定したトレイトであり、必ずしも無作為ではないことに注意が必要である。無作為抽出が必要な場合は`random`モジュールを使用する。

主要な値クラスは全て`Sample`を実装する。また、`Sample`なクラスで構成されているタプル型やレコード型、Or型、篩型でも実装されている。

```erg
assert Int.sample() == 42
assert Str.sample() == "example"
# Intの場合、標準では64bitの範囲でサンプルされる
print! Int.sample!() # 1313798
print! {x = Int; y = Int}.sample!() # {x = -32432892, y = 78458576891}
```

以下は`Sample`の実装例である。

```erg
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
