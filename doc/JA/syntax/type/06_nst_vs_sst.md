# 記名的部分型 vs. 構造的部分型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/06_nst_vs_sst.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/06_nst_vs_sst.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

```erg
Months = 0..12

# NST
MonthsClass = Class Months
MonthsClass.
    name self =
        match self:
            1 -> "January"
            2 -> "February"
            3 -> "March"
            ...

# SST
MonthsImpl = Patch Months
MonthsImpl.
    name self =
        match self:
            1 -> "January"
            2 -> "February"
            3 -> "March"
            ...

assert 12 in Months
assert 2.name() == "February"
assert not 12 in MonthsClass
assert MonthsClass.new(12) in MonthsClass
# クラスでラップしても構造型は使える
assert MonthsClass.new(12) in Months
# 両方ある場合クラスメソッドが優先
assert MonthsClass.new(2).name() == "february"
```

## 結局、NSTとSSTどちらを使えばいいのか？

どちらにすればよいか判断がつかないときはNSTを推奨します。
SSTはどんなユースケースでも破綻しないコードを書く抽象化能力が要求されます。よい抽象化を実現できれば高い生産性を発揮できますが、間違った抽象化(__見かけによる共通化__)を行うと逆効果となってしまいます。NSTは抽象性をあえて抑え、このリスクを減らすことができます。あなたがライブラリの実装者でないならば、NSTのみでコーディングを行っても悪くはないでしょう。

<p align='center'>
    <a href='./04_class.md'>Previous</a> | <a href='./06_inheritance.md'>Next</a>
</p>
