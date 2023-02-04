# コミットメッセージに関するガイドライン

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/commit_message.md%26commit_hash%3De2469cc0df18d3e3a01d9b483fcd7bfd7ddbe54c)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/commit_message.md&commit_hash=e2469cc0df18d3e3a01d9b483fcd7bfd7ddbe54c)

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

* `type`はcommitの型を表します。小文字で書いてください(自動commitは大文字で始まるので、これによって手動コミットかどうかを区別します)。`feat`は新しい機能、`fix`はバグの修正やissueの解決、`docs`はドキュメントの変更、`style`はコードスタイルの変更、`refactor`はリファクタリング、`perf`はパフォーマンスの改善、`test`はテストの追加や変更、`build`はビルド関連・バージョン・依存関係の変更、`ci`はCI関連の変更、`chore`は内部的・軽微な変更、`revert`はrevertです。複数該当する場合は、より具体的なtypeを選んでください。優先度の低いtypeは`fix`, `refactor`, `style`, `chore`になります。例えば、ドキュメント(docs)の修正(fix)は`docs`、テスト(test)のリファクタリング(refactor)は`test`になります。

* `scope`は省略可能で、コミットの影響範囲を表します。例えば、`fix(parser):`というコミットメッセージはパーサーのバグ修正であることを示します。コンマ区切りで複数のスコープを指定することもできますが、その場合コミットを分割することも検討してください。スコープの例は以下の通りです。

  * `parser`
  * `compiler`
  * `els`

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

* 作業途中のコミットは自由に書いて構いません。最終的にsquash等をして整理するときに、規則に従ってください。
* 文は特に理由のない場合現在形・現在進行形で書いてください。
