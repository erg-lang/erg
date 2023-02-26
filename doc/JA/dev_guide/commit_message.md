# コミットメッセージに関するガイドライン

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/commit_message.md%26commit_hash%3D718ae9d7d8118fcf5f36561ebbcfa96af980ec32)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/commit_message.md&commit_hash=718ae9d7d8118fcf5f36561ebbcfa96af980ec32)

このガイドラインは、

* コミットメッセージをどうやって書けば良いのかを明確化する
* コミットメッセージを後から参照しやすくする

などを目的としています。努力目標であり、フォーマットから外れたコミットをした場合でもrebaseして修正などを要求するものではありません(直前のコミットメッセージを変更したい場合は`--amend`オプションが使えます)。

あなたが守るべきガイドラインは以下の2点です。

* 自動的につけられたコミットメッセージ(e.g. `Update xxx.md`, `Automatic update xxx`, `Merge pull request #XXX ...`)は手を加えずそのまま送る
* 手動コミットのメッセージは[conventional commits](https://www.conventionalcommits.org/ja/v1.0.0/#%e4%bb%95%e6%a7%98)に準拠する

conventional commitsのBNFは以下のようになっています。

```bnf
commit ::= type ('(' scope ')')? '!'? ':' description body? footer*
type ::= 'feat' | 'fix' | 'docs' | 'style' | 'refactor' | 'perf' | 'test' | 'build' | 'ci' | 'chore' | 'revert'
```

我々はGitHub上で開発するので、これを少し拡張して、issue/PR番号をdescriptionの後に追加して良いことにします。

```bnf
commit ::= type ('(' scope ')')? '!'? ':' description ('(' '#' issue ')')? body? footer*
```

各部分の意味は以下の通りです。

* `type`はcommitの型を表します。小文字で書いてください(自動commitは大文字で始まるので、これによって手動コミットかどうかを区別します)

| type       | 説明                                   |
| ---------- | -------------------------------------- |
| `feat`     | 新しい機能                             |
| `fix`      | バグの修正やissueの解決                |
| `docs`     | ドキュメントの変更                     |
| `style`    | コードスタイルの変更                   |
| `refactor` | リファクタリング                       |
| `perf`     | パフォーマンスの改善                   |
| `test`     | テストの追加や変更                     |
| `build`    | ビルド関連・バージョン・依存関係の変更 |
| `ci`       | CI関連の変更                           |
| `chore`    | 内部的・軽微な変更                     |
| `revert`   | revert                                 |

複数該当する場合は、より具体的なtypeを選んでください。優先度の低いtypeは`fix`, `refactor`, `style`, `chore`になります。例えば、ドキュメント(docs)の修正(fix)は`docs`、テスト(test)のリファクタリング(refactor)は`test`になります。
なお、ユーザーに表示されるメッセージの改善は`fix`になります。Erg開発チームは分かりにくい・的を外したメッセージをバグとして扱います。

* `scope`は省略可能で、コミットの影響範囲を表します。例えば、`fix(parser):`というコミットメッセージはパーサーのバグ修正であることを示します。コンマ区切りで複数のスコープを指定することもできますが、その場合コミットを分割することも検討してください。スコープの例は以下の通りです。

  * `parser`
  * `compiler`
  * `typechecker`
  * `els`
  * `REPL`
  * `linter`

* `!`マークはコミットが破壊的な変更であることを示します。このマークがついている場合、破壊的変更の理由を書く必要があります。破壊的変更は、言語の仕様変更やコンパイラAPIの変更などが該当します。

* `description`はコミットの概要を表します。あまり短すぎてはいけませんが、おおよそ50文字以内に収めるべきです。原則として英語で書いてください。大文字の単語で始まるとき以外は小文字で始めてください。ピリオドは付けないでください。

* `body`は省略可能で、コミットの詳細を表します。

* `footer`は省略可能で、コミットの関連情報を表します(レビュアーの一覧や、関連するissue/PR番号を書くなど)。

---

例としては以下になります。

```txt
feat(parser): add support for foo (#123)
```

```txt
fix: address CVE-XXXX-YYYY

Ref: https://cve.mitre.org/...
```

```txt
docs!: remove `xxx.md`

The contents of xxx.md are old and inaccurate, so it is deleted.
```

```txt
docs: update commit hash of `xxx.md`
```

```txt
refactor(compiler): `Foo` => `FooBar`
```

```txt
build: update version (v0.1.2 => v0.1.3)
```

```txt
style: fix typo
```

例から分かる通り、APIやファイル・ディレクトリ名は``で囲ってください。

## 補足

作業途中のコミットは自由に書いて構いません。最終的にsquash等をして整理するときに、規則に従ってください。
文は特に理由のない場合現在形・現在進行形で書いてください。
PRに乱雑なコミットがある場合は、PR名を変更し(commit_message仕様を使用)、squash and mergeを使用してください(コミットが明確な場合は直接マージしてください)

## テンプレートの設定

もしテンプレートを利用したい場合は以下のコマンドを利用してください

```shell
git config commit.template .gitmessage
```

これによりErgのリポジトリ内でのみこのコミットメッセージのテンプレートが利用されます

```txt
# type(scope): description (#issue)

# body
# Wrap at 72 chars. ################################## which is here:  #
#
# footer
# Wrap at 72 chars. ################################## which is here:  #
#
########################################################################
#
# ## Help ##
#
# ## type: must ##
# feat: new feature
# fix: bug fix or issue resolution
# docs: documentation changes
# style: code style changes
# refactor: refactoring
# perf: performance improvement
# test: adding or changing tests
# build: build-related/version/dependency
# ci: CI-related changes
# chore: internal/minor changes
# revert: revert commit
# * fix, refactor, style and chore are lower priority
#
# ## scope: optional ##
# Indicates the scope
# e.g.
# - parser
# - compiler
# - els
# - REPL
# - linter
#
# ## !: optional ##
# Destructive change
#
# ## description: must ##
# Summary of the commit
# No more than 50 chars
#
# ## issue: optional ##
# Related issue/PR number
#
# ## body: optional ##
# Indicates the details of the commit
#
# ## footer: optional ##
# Represents information related to the commit
```
