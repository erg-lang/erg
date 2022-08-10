# The Erg Programming Language

<div align="center">
    <img width="500" src="./assets/erg_logo_with_slogan.svg">
</div>

<br>[Erg](https://mtshiba.github.io/TheErgBook)はPython互換の静的型付け言語です。

<p align='center'>
    <img alt="Build status" src="https://github.com/erg-lang/erg/actions/workflows/rust.yml/badge.svg">
    <a href="https://opensource.org/licenses/Apache-2.0"><img alt="License: MIT & APACHE 2.0" src="https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue"></a><br>
    <a href='./README.md'>English</a> | 日本語
</p>

## Ergはこんな人におすすめです&#58;

* Pythonを使用しているが、Rustのような静的型付き言語で堅牢かつ快適にコーディングしたい
* しかし、煩雑な型定義やメモリ管理は避けたい
* MLのようにシンプルで一貫性のある言語を使いたい
* 依存型/篩型を持つ実用的な汎用言語を使いたい
* Scalaのように関数型とオブジェクト指向が高度に融合された言語を使いたい

## 特徴

> いくつかの機能は未実装です。実装状況は[TODO.md](./TODO.md)を御覧ください。

1. 堅牢性

    Ergは賢くパワフルな型システムを持っています。コンパイル時のnullチェック(Option型)はもちろん可能で、ゼロ除算や配列の範囲外アクセスまでもがコンパイル時に検出できます。

    ```python
    rand = import "random"

    l = [1, 2, 3]
    assert l in [Int; 3] # 型チェック
    assert l in [1..3; 3] # さらに詳細に
    l2 = l.push(rand.choice! 0..10)
    assert l2 in [0..10; 4]
    assert l2 + [3, 5, 7] in [0..10; 7]
    # これはIndexErrorを引き起こしますが、コンパイル時に検出できます
    l2[10] # IndexError: `l2`は7つの要素を持っていますが、10番目の要素のアクセスしようとしています

    2.times! do!:
        print! "hello, ", end: ""
    # => hello, hello, 
    -2.times! do!:
        print! "hello,", end: ""
    # TypeError: `.times!`は`Nat`(0以上のInt)のメソッドです、`Int`ではありません

    {Meter; Sec; meter; yard; sec; ...} = import "unit"

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

    Ergでは特別扱いされる構文要素がとても少なく、例えば予約語が一つもありません。以下のような芸当も可能です。

    ```python
    loop! block = while! True, block

    # `while! True, do! print! "hello"`と同じです
    loop! do!:
        print! "hello"
    ```

3. 関数型&オブジェクト指向

    Ergは純粋なオブジェクト指向言語です。全てはオブジェクトであり、型、関数、演算子も例外ではありません。
    一方、Ergは関数型言語でもあります。副作用を引き起こすコードには`!`を付けなくてはなりません。これは、副作用の局所化を意識させてくれます。

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
    {sin; pi; ...} = math
    # Pythonの外部モジュールを使います
    Tqdm! = pyimport("tqdm").'tqdm'

    print! sin pi # 1.2246467991473532e-16
    for! Tqdm!.'__call__'(0..99), i =>
        time.sleep! 0.01 * i
    ```

5. 読みやすいエラーメッセージ

    Ergはエラーメッセージの読みやすさを重視しています。Ergはプログラマに寄り添う言語であり、~~C++のように~~訳のわからない呪文を吐いたりはしません。

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
    ヒント: `Array`は`push`メソッドを持っています、詳しくは https://erg-lang.org/docs/prelude/Array/##push を参照してください
    ヒント: `Array!`は`push!`メソッドを持っています、詳しくは https://erg-lang.org/docs/prelude/Array!/##push! を参照してください
    ```

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

## コントリビューション

コントリビューション(プロジェクトへの貢献、協力)はいつでも歓迎しています！
何かわからないことがあれば、[Discordチャンネル](https://discord.gg/tnFTmANNkg)で気軽に質問してください。

## ライセンス

Ergは、MITライセンスとApache2.0ライセンスのデュアルライセンスで配布されています。
詳しくは[LICENSE-APACHE](./LICENSE-APACHE), [LICENSE-MIT](./LICENSE-MIT)をご覧ください。
