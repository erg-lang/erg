# Erg への貢献

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3DCONTRIBUTING.md%26commit_hash%3D00350f64a40b12f763a605bc16748d09379ab182)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=CONTRIBUTING.md&commit_hash=00350f64a40b12f763a605bc16748d09379ab182)

初心者は[こちら](https://github.com/erg-lang/erg/issues/31#issuecomment-1217505198)の説明を読んでください。

## ドキュメント

Erg への貢献を考えている場合は、`doc/*/dev_guide` にあるドキュメントを読む必要があります。特に`env.md`に書かれているものを事前にインストールしてください。

Erg の内部構造に興味がある場合は、`doc/*/compiler` が役に立つかもしれません。

## バグレポート

Ergのバグだと思われる動作を見つけた場合は、[報告](https://github.com/erg-lang/erg/issues/new/choose)していただければ幸いです。同じバグがまだissueとして報告されていないことを確認してください。

`cargo run --features debug`と入力すると、Erg はデバッグモードでビルドされます。このモードでは、バグの調査に役立つ情報がダンプされる場合があります。このモードでエラーログを報告していただければ幸いです。

また、バグの原因が環境によらない場合は、バグが発生した環境を報告する必要はありません。

## ドキュメントの翻訳

私たちは常に、ドキュメントをさまざまな言語バージョンに翻訳してくれる人を探しています。

ドキュメントが他の言語に比べて古くなっていることに気づき、内容を更新したいという方も歓迎します ([こちら](https://github.com/erg-lang/erg/issues/48#issuecomment-1218247362) を参照)。これを行う方法について)。

## 質問する

ご不明な点がございましたら、[Discord チャンネル](https://discord.gg/zfAAUbgGr4)までお気軽にお問い合わせください。

## 開発・実装に関して

リクエストは常に受け付けますが、常に採用されるとは限らないと心に留めておいてください。多くの問題には、トレードオフが存在します。

他者がアサインされたイシューを横取りするのはやめましょう(GitHubでassigneesを確認してください)。一人では手に余ると判断された場合は、さらに応援を募ります。

機能の提案をする前に、その機能が既存の機能を組み合わせて容易に解決できないか考えてください。

Ergチームや言語で標準とされるスタイルのコードを書いてください。

## [行動規範](./../CODE_OF_CONDUCT/CODE_OF_CONDUCT_JA.md)
