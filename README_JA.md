# The Erg Programming Language

<div align="center">
    <img width="500" src="./assets/erg_logo_with_slogan.svg">
</div>

<br>こちらは[Erg](https://erg-lang.org/)のメインリポジトリです。コンパイラとドキュメントが置かれています。

<p align='center'>
    <a href="https://github.com/erg-lang/erg/releases"><img alt="Build status" src="https://img.shields.io/github/v/release/erg-lang/erg.svg"></a>
    <a href="https://github.com/erg-lang/erg/actions/workflows/main.yml"><img alt="Build status" src="https://github.com/erg-lang/erg/actions/workflows/main.yml/badge.svg"></a>
<br>
    <a href="https://erg-lang.org/web-ide/" data-size="large">
        <img src="https://img.shields.io/static/v1?style=for-the-badge&label=&message=playground&color=green">
    </a>
</p>

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3DREADME.md%26commit_hash%3D718ae9d7d8118fcf5f36561ebbcfa96af980ec32)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=README.md&commit_hash=718ae9d7d8118fcf5f36561ebbcfa96af980ec32)

## Ergはこんな人におすすめです&#58;

* Rustのような静的型付き言語で堅牢かつ快適にコーディングしたい
* しかし、煩雑な型定義やメモリ管理は避けたい
* Pythonに不満があるが、Pythonのコード資産を捨てきる決心が付かない
* MLのようにシンプルで一貫性のある言語を使いたい
* 依存型/篩型を持つ実用的な汎用言語を使いたい
* Scalaのように関数型とオブジェクト指向が高度に融合された言語を使いたい

## 特徴

> いくつかの機能は未実装です。実装状況は[TODO.md](./TODO.md)を御覧ください。

1. 堅牢性

    Ergは賢くパワフルな型システムを持っています。コンパイル時のnullチェック(Option型)はもちろん可能で、ゼロ除算や配列の範囲外アクセスまでもがコンパイル時に検出できます。

    ```python
    rand = pyimport "random"

    l = [1, 2, 3]
    assert l in [Int; 3] # 型チェック
    assert l in [1..3; 3] # さらに詳細に
    l2 = l.push(rand.choice! 0..10)
    assert l2 in [0..10; 4]
    assert l2 + [3, 5, 7] in [0..10; 7]
    # これはIndexErrorを引き起こしますが、コンパイル時に検出できます
    l2[10] # IndexError: `l2`は7つの要素を持っていますが、11番目の要素のアクセスしようとしています

    2.times! do!:
        print! "hello, ", end := ""
    # => hello, hello,
    -2.times! do!:
        print! "hello,", end := ""
    # TypeError: `.times!`は`Nat`(0以上のInt)のメソッドです、`Int`ではありません

    {Meter; Sec; meter; yard; sec} = import "unit"

    velocity x: Meter, t: Sec = x / t

    v = velocity 3yard, 2sec # TypeError: `x`の型が適合しません。`Meter`を予期しましたが、`Yard`が渡されました
    v = velocity 3meter, 2sec # v == 1.5 m/s
    ```

2. 簡潔性

    Ergはとてもシンプルな文法からなり、コードもシンプルに書き上げられます。しかし、その機能の豊富さは他の言語に劣りません。
    型推論機構は非常に強力であり、まるで動的型付け言語かのように書くことができます。

    ```python
    fib 0 = 0
    fib 1 = 1
    fib n = fib(n - 1) + fib(n - 2)
    assert fib(10) == 55
    ```

    for式やwhile式もサブルーチンの一つに過ぎません。以下のような芸当も可能です。

    ```python
    loop! block! = while! do! True, block!

    # `while! do(True), do! print! "hello"`と同じです
    loop! do!:
        print! "hello"
    ```

3. 関数型&オブジェクト指向

    Ergは純粋なオブジェクト指向言語です。全てはオブジェクトであり、型、関数、演算子も例外ではありません。
    一方、Ergは関数型言語でもあります。Ergでは副作用を引き起こすコードや内部状態を変更するコードに、ある種の記号を書く必要があり、これによりコードの複雑さを局所化することができます。これにより、コードのメンテナンス性が大幅に向上します。

    ```python
    # immutableな関数型スタイル、Pythonの`.sorted()`と同じです
    immut_arr = [1, 3, 2]
    assert immut_arr.sort() == [1, 2, 3]
    # mutableなオブジェクト指向スタイル
    mut_arr = ![1, 3, 2]
    mut_arr.sort!()
    assert mut_arr == [1, 2, 3]
    i = !1
    i.update! old -> old + 1
    assert i == 2

    # 関数は副作用を起こせません
    inc i: Int! =
        i.update! old -> old + 1
    # SyntaxError: 関数の中でプロシージャルメソッドは呼び出せません
    # ヒント: 可変型メソッドだけがオブジェクトの状態を変更できます

    # 副作用を多用するコードは自然と冗長になるため、自然と純粋なコードを書くように動機づけられます
    Counter! = Inherit Int!
    Counter!.
        new i: Int = Self!::__new__ !i
        inc! ref! self =
            self.update! old -> old + 1

    c = Counter!.new 1
    c.inc!()
    assert c == 2
    ```

4. 相互運用性

    Ergは内部的にPythonと互換性があり、PythonのAPIをゼロコストで呼び出すことが出来ます。

    ```python
    # Pythonのビルトインモジュールを使います
    math, time = pyimport "math", "time"
    {sin; pi} = math
    # Pythonの外部モジュールを使います
    tqdm = pyimport("tqdm")

    print! sin pi # 1.2246467991473532e-16
    for! tqdm.tqdm(0..99), i =>
        time.sleep! 0.01 * i
    ```

5. 読みやすいエラーメッセージ

    Ergはエラーメッセージの読みやすさを重視しています。~~C++とは異なり~~Ergはプログラマに寄り添う言語です。

    ```python
    proc! x =
        l = [1, 2, 3]
        l.push!(x)
        l
    ```

    ```console
    Error[#12]: ファイル example.er, 3行目, <module>::proc!
    2│     l = [1, 2, 3]
    3│     l.push!(x)
             ^^^^^
    AttributeError: Arrayオブジェクトは`.push!`という属性を持っていません
    ヒント: オブジェクトの内部状態を変更したい場合は、`!`演算子を使って可変化してください
    ヒント: `Array`は`push`メソッドを持っています、詳しくは https://erg-lang.github.io/docs/prelude/Array/##push を参照してください
    ヒント: `Array!`は`push!`メソッドを持っています、詳しくは https://erg-lang.github.io/docs/prelude/Array!/##push! を参照してください
    ```

## Requirements

[Python3 (3.7~3.11)](https://www.python.org/)インタープリタがインストールされている必要があります。すでにインストールされているならセットアップは不要です。

REPLでペースト機能を利用したい場合は`xsel`をインストールしてください(WSLでは必要ありません)

## インストール

### cargo(Rustパッケージマネージャ)によるインストール

```sh
cargo install erg
```

### ソースコードからのビルド

ソースコードからのビルドにはRustツールチェインが必要です。

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
cargo build --release
```

### Nixによるビルド

[Nix](https://nixos.org/)をインストールしているなら，次のコマンドでプロジェクト直下の`result/bin/erg`にバイナリが生成されます．

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix-build
```

[Nix Flakes](https://nixos.wiki/wiki/Flakes)を有効化している場合は次のコマンドでも良いです.

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix build
```

### フラグ

`--features`フラグを有効化することで、インストールとビルドをカスタマイズできます。

* もしエラーメッセージの言語を変えるには`--features {language}`というオプションを使用します。

```sh
cargo install erg --features japanese
cargo install erg --features simplified_chinese
cargo install erg --features traditional_chinese
```

さらに多くの言語に対応する予定です。(翻訳者を募集しています。ぜひ[翻訳プロジェクト](./doc/JA/dev_guide/i18n_messages.md)に参加してください)。

その他のフラグ

* ELS (Erg Language Server) を有効にする
  * `--features els`
* デバッグモード (コントリビューター向け)
  * `--features debug`
* REPLの履歴機能やペースト機能を有効にする
  * `--features full-repl`
* エラー表示などの見栄えを良くする
  * `--features unicode`と`--features pretty`
* 全機能を有効にする (開発者用機能を除く)
  * `--features full`
* その他のフラグ・エイリアスは[こちら](https://github.com/erg-lang/erg/blob/main/.cargo/config.toml)をご覧ください。

## コントリビューション

コントリビューション(プロジェクトへの貢献、協力)はいつでも歓迎しています！始めたい方は[こちら](https://github.com/erg-lang/erg/blob/main/doc/CONTRIBUTING/CONTRIBUTING_JA.md)をお読みください。

何かわからないことがあれば、[Discordチャンネル](https://discord.gg/zfAAUbgGr4)で気軽に質問してください。

## License

[assets](./assets)、および[doc](./doc)内のすべてのファイルは、[CC-BY-4.0](./doc/LICENSE)でライセンスされています。残りのファイルは、[Apache License 2.0](./LICENSE-APACHE) + [MIT License](./LICENSE-MIT) でライセンスされています。

サードパーティーの依存ライブラリのクレジットについては、[THIRD_PARTY_CREDITS.md](./THIRD_PARTY_CREDITS.md) (英語) をご覧ください。
