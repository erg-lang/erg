# パッケージマネージャー

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/pack.md%26commit_hash%3D8dcbcb4235ba73cd2618fe5407a1ea18f7784da1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/pack.md&commit_hash=8dcbcb4235ba73cd2618fe5407a1ea18f7784da1)

Ergは標準でパッケージマネージャーが付属しており、`pack`サブコマンドで呼び出せる。
以下は典型的なオプションである。

* `erg pack init`: 現在のディレクトリをパッケージとして初期化する。`package.er`ファイルや`src`ディレクトリが生成される。`app`と指定すると実行ファイルのパッケージ、`lib`と指定するとライブラリのパッケージ、`hybrid`を指定すると両方のパッケージとなる。`--license`を指定すると自動でライセンスファイルを置いてくれる。
* `erg pack build`: パッケージをビルドする。`--release`をつけるとテストが実行され、最適化をする。成果物は`build/debug`か`build/release`に配置される。
* `erg pack install`: パッケージをインストールする。ライブラリの場合は`.erg/lib`に`src`以下が置かれ、アプリケーションは`.erg/app`にシェルスクリプトとして置かれる。`--release`をつけると最適化をする。
* `erg pack run`: パッケージをビルドしてアプリケーションを実行する(appパッケージのみ)。
* `erg pack clean`: buildディレクトリの中身を削除します。
* `erg pack test`: パッケージのテストを行う。詳しくは[test.md](./test.md)を参照。
* `erg pack publish`: パッケージを公開/リリースします。GitHubのアカウントと公開鍵が必要です。

なお、このドキュメントでは自前のパッケージを管理する際の方法を説明する。
外部パッケージをインストールしたり検索したりしたい場合は[install.md](./install.md)を参照。
また、Ergのパッケージシステムについては[package_system.md](../syntax/35_package_system.md)を参照。

## パッケージ全体の標準ディレクトリ構成(アプリケーションパッケージの場合)

```console
/package # パッケージのルートディレクトリ
    /build # ビルド結果を格納するディレクトリ
        /debug # デバッグビルド時の成果物
        /release # リリースビルド時の成果物
    /doc # ドキュメント(さらに`en`, `ja`などのサブディレクトリに分けることで各国語対応可能)
    /src # ソースコード
        /main.er # main関数を定義するファイル
    /tests # (ブラックボックス)テストファイルを格納するディレクトリ
    /package.er # パッケージの設定を定義するファイル
```

## package.er

`erg pack init`すると以下のようなファイル、`package.er`が生成される。`package.er`にはパッケージの設定を記述する。
以下は`package.er`の記述例である。

```python
name = "example" # package name
author = "John Smith" # package author name
version = "0.1.0"
description = "An awesome package"
categories = ["cli"] # package categories
type = "app" # "app" or "lib"
license = "" # e.g. "MIT", "APACHE-2.0", "MIT OR Apache-2.0"
pre_build = "" # script filename to be executed before build
post_build = "" # script filename to be executed after build
dependencies = {
    # The latest one is selected if the version is not specified
    # If the version specification is omitted, the package manager automatically adds the version of the last successful build to the comments
    foo  = pack("foo") # [INFO] the last successfully built version: 1.2.1
    # Packages can be renamed
    bar1 = pack("bar", "1.*.*") # [INFO] the last successfully built version: 1.2.0
    bar2 = pack("bar", "2.*.*") # [INFO] the last successfully built version: 2.0.0
    baz  = pack("baz", "1.1.0")
}
deprecated = False
successors = [] # alternative packages (when a package is deprecated)
```

## セマンティックバージョニング

Ergのパッケージは[セマンティックバージョニング](https://semver.org/lang/ja/)に基づいてバージョンの指定を行います。
セマンティックバージョニングとは、大まかには`x.y.z`(x,y,zは0以上の整数)の書式で指定されるバージョニングです。
それぞれの数字の意味は以下のようになります。

* x: メジャーバージョン(互換性を破壊する更新を行うとき1上げる)
* y: マイナーバージョン(互換性のある更新(API追加・非推奨化など)を行うとき1上げる、バグ修正などはパッチバージョンアップで対応する)
* z: パッチバージョン(バグ修正・互換性を保つ軽微な変更を行うとき1上げる、互換性を破壊する深刻な修正はメジャーバージョンアップで対応する)

ただしバージョン`0.*.*`の変更はデフォルトで常に互換性がありません。互換性を保ったままバージョンアップしたい場合は後ろに`-compatible`と指定します(Erg独自ルール)。例えば、`0.2.1`を互換性を保ったまま機能追加したい、すなわち`0.3.0`にバージョンアップしたい場合は`0.3.0-compatible`と指定します。またバグフィックスを行った場合は`0.2.2-compatible`と指定します。
こうすると、そのバージョンは直前のバージョンと互換性があると見なされるようになります。
これは`0.*.*`を`1.0.0`にバージョンアップしたい場合でも使えます。すなわち、`1.0.0-compatible`は直前のバージョン`0.y.z`と互換性があります。

セマンティックバージョニングはロックファイルを生成する際非常に重要です。ロックファイルは依存パッケージの互換性を保つために生成されるファイルで、依存パッケージの新しいリリースがあっても明示的にアップデートしない限り古いパッケージに依存します。
ロックファイルは依存パッケージのあるパッケージを複数人で開発する際に便利です。また、依存パッケージがさらに依存するパッケージについて、互換性があるならばパッケージを使いまわすことができるので、ローカルストレージの節約にもなります。

Ergのパッケージマネージャは以上のルールを厳密に適用しており、ルールに抵触するパッケージ更新は拒絶されます。
Ergパッケージマネージャはバージョン管理システム(git等)と連携しており、パッケージのpublish時にコードの差分を検知し、バージョニングの正当性を検証します。
具体的に言うと、パッケージマネージャはAPIの型を見ます。型が古いバージョンのサブタイプになっていれば、変更は互換性があるとみなされます(これは完全な検証ではないことに注意してください。型的には互換でも意味論的に非互換な変更はあり得ます。これを判断するのは開発者の仕事です)。

さらにパッケージはリポジトリ全体がレジストリに登録されるため、開発者であってもパッケージマネージャを通さずにパッケージの更新をすることは出来ません。
また、パッケージは非推奨にはできても削除することはできません。

### Appendix: セマンティックバージョニングの問題と、その対策

セマンティックバージョニングには既知の問題が(少なくとも)2つあります。
まず、セマンティックバージョニングは過大な制約を課す可能性があります。
セマンティックバージョニングでは、たった1つの非互換なAPI変更でパッケージ全体のメジャーバージョンが上がってしまいます。
こうなると、「新しいAPIを試したかったが、別の非互換なAPI変更に対処しなくてはならないのでバージョンアップを見送る」といったことが発生します。
もう一つ、セマンティックバージョニングは過大な約束をする可能性があります。
前項で述べたように、APIの「互換性ある変更」は理論的に証明できるものではありません。バージョン`1.0.1`のパッケージがほしいと指定した場合、セマンティックバージョニングの観点では`1.0.1`以上`2.0.0`未満のパッケージ全てを代わりに使うことができます(`1.0.0`は使えません。バグ修正が入ったからです)が、実際はパッケージ開発者の意図しないAPI利用によってビルドが成功しない可能性があります。

Ergではこの問題に対処するため、別のバージョンのパッケージを(リネームすることで)同時に利用することができるという方策を取っています。これによって、ver2のAPIを一部導入しながら、ver1のAPIも引き続き利用するといった事が可能になります。
さらに、あまり望ましい状態ではありませんが、ある特定のマイナーバージョンのAPIのみがバグなしに使えるといった場合にそれだけを残して次のバージョンへ進むことが可能です。

## publish

`publish`サブコマンドでパッケージの公開が可能です。公開にはGitHubアカウントが必要です。
パッケージはデフォルトでは`(owner_name)/(package_name)`で登録されます。一定の条件(ダウンロード数、メンテナンスの頻度など)を満たすとオーナー名を省略したエイリアスを登録する申請が出来ます。
なおパッケージ名の大文字/小文字や`_`, `-`などの区切り文字は区別されません。

パッケージは、再現性を保証するためにレジストリに保存されます。基本的に、一度アップロードした内容は変更・削除できないので注意してください。更新は新バージョンの公開のみによって行えます。
