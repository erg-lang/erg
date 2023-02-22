# バージョン

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/version.md%26commit_hash%3Dbaf9e9597fbe528ed07a354a2b145e42ceef9e42)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/version.md&commit_hash=baf9e9597fbe528ed07a354a2b145e42ceef9e42)

Ergコンパイラはセマンティックバージョニングに従ってバージョン番号を付ける。
ただし、バージョン0の間は通常と異なるルールが適用される(セマンティックバージョニングよりも細かいルールに従う)。
ここで注意すべき点として、Ergには2種類の互換性がある。言語仕様の互換性を示す __仕様互換性__ と、コンパイラ等の(公開)APIの互換性を示す __内部互換性__ である。

* バージョン0の間は、マイナーリリースで仕様・内部互換性が壊れる可能性がある。これは通常のセマンティックバージョニングと同じである。
* バージョン0の間は、パッチリリースで仕様互換性が壊れることはないが、内部互換性の保証はない。
* 新機能は主にマイナーリリースで追加されるが、些細な言語機能である場合、またはコンパイラの機能の場合はパッチリリースでも追加されうる。

## リリースサイクル

* パッチリリースは1~2週間に一度程度行われる。ただし重篤なバグが発見された場合、前倒しされる可能性もある。
* マイナーリリースはパッチリリース10数回程度、すなわち3~6ヶ月に一度行われる。
* メジャーリリースの間隔は不定である。バージョン1リリースの予定は現在のところ定まっていない。

## nightly, betaリリース

Ergでは不定期にnightlyリリースとbetaリリースを行う。nightlyリリースは新しいパッチリリースの先行公開版であり、betaリリースは新しいマイナー/メジャーリリースの先行公開版である。
nightlyバージョン及びbetaバージョンはcrates.io上で公開される。betaバージョンの場合はGitHub releasesでも公開される。

nightlyバージョンの形式は`0.x.y-nightly.z`である。同一のパッチリリースに対して複数のnightlyリリースが存在することがあり、その場合は新しくなるにつれて`z`が増加する。betaバージョンも同様である。

nightlyリリースはほぼ隔日で行われる(コンパイラ本体に対する変更のない日は行われない)が、betaリリースは不定期である。ただし、betaリリースは一度公開されると、ほぼ隔日で新しいbetaリリースが公開される。
