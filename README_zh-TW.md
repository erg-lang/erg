# The Erg Programming Language

<div align="center">
    <img width="500" src="./assets/erg_logo_with_slogan.svg">
</div>

<br>這是[Erg](https://erg-lang.org/)的主要源代碼庫。它包含編譯器和文檔

<p align='center'>
    <a href="https://github.com/erg-lang/erg/releases"><img alt="Build status" src="https://img.shields.io/github/v/release/erg-lang/erg.svg"></a>
    <a href="https://github.com/erg-lang/erg/actions/workflows/main.yml"><img alt="Build status" src="https://github.com/erg-lang/erg/actions/workflows/main.yml/badge.svg"></a>
<br>
    <a href="https://erg-lang.org/web-ide/" data-size="large">
        <img src="https://img.shields.io/static/v1?style=for-the-badge&label=&message=playground&color=green">
    </a>
</p>

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3DREADME.md%26commit_hash%3D718ae9d7d8118fcf5f36561ebbcfa96af980ec32)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=README.md&commit_hash=718ae9d7d8118fcf5f36561ebbcfa96af980ec32)

## Erg可以推薦給以下人員&colon;

* 希望有類似Rust的健壯性和舒適的編譯器支持，然而，又不需要像Rust那樣的冗長的類型規範和內存管理模型
* 對Python不滿意，但無法下定決心放棄Python代碼資產
* 希望有一個像ML那樣簡單而一致的語言
* 希望有一個實用的通用語言，有依賴/反射類型
* 想要一種像Scala一樣的語言，既面向對象，又使用函數式編程

## 特征

> 某些功能尚未實現。有關實施情況請查看 [TODO.md](./TODO.md) 了解實施情況

1. 穩健性

    Erg有一個智能而強大的類型系統。例如: Erg 可以在編譯時檢查: 空值（Option類型）、除以零的情況、數組中的超出範圍的地址

    ```python
    rand = pyimport "random"

    l = [1, 2, 3]
    assert l in [Nat; 3] # 類型檢查
    assert l in [1..3; 3] # 更詳細
    l2 = l.push(rand.choice! 0..10)
    assert l2 in [0..10; 4]
    assert l2 + [3, 5, 7] in [0..10; 7]
    # 這將導致下標錯誤，Erg可以在編譯時發現它
    l2[10] # 下標錯誤: `l2`只有7個元素，但卻被訪問了第11個元素

    2.times! do!:
        print! "hello, ", end := ""
    # => hello, hello,
    -2.times! do!:
        print! "hello, ", end := ""
    # 類型錯誤: `.times!`是`Nat`(0或更大整數)的方法，不是`Int`的

    {Meter; Sec; meter; yard; sec} = import "unit"

    velocity x: Meter, t: Sec = x / t

    v = velocity 3yard, 2sec # 類型錯誤: `x`的類型不匹配: 預期為`Meter`，找到`Yard'`
    v = velocity 3meter, 2sec # v == 1.5 m/s
    ```

2. 簡潔性

    Erg由一個非常簡單的語法組成，與其他語言相比，它可以大大減少代碼量。然而，其功能並不遜色於它們

    由於類型推理系統很強大，你可以像動態類型語言一樣編碼

    ```python
    fib 0 = 0
    fib 1 = 1
    fib n = fib(n - 1) + fib(n - 2)
    assert fib(10) == 55
    ```

    甚至for和while表達式也只是子程序之一，所以這是可能的

    ```python
    loop! block! = while! do! True, block!

    # equals to `while! do(True), do! print! "hello"`
    loop! do!:
        print! "hello"
    ```

3. 函數式 & 面向對象

    Erg是一種純面向對象的語言，一切都是對象。類型，函數和運算符都是對象。另一方面，Erg也是一種函數式語言
    Erg要求在引起副作用或改變內部狀態的代碼上放置某些種類的標記，這可以使代碼的復雜性局部化，這將大大改善代碼的可維護性

    ```python
    # 函數式風格（不可變），與Python中的`sorted(list)`相同
    immut_arr = [1, 3, 2]
    assert immut_arr.sort() == [1, 2, 3]
    # Object-oriented style (mutable)
    mut_arr = ![1, 3, 2]
    mut_arr.sort!()
    assert mut_arr == [1, 2, 3]
    i = !1
    i.update! old -> old + 1
    assert i == 2

    # 函數不能引起副作用
    inc i: Int! =
        i.update! old -> old + 1
    # 語法錯誤: 不能在函數中調用程序性方法
    # 提示: 只有可變類型的方法才能改變對象的狀態

    # 使用大量副作用的代碼是多余的，所以你自然會寫純代碼
    Counter! = Inherit Int!
    Counter!.
        new i: Int = Counter! !i
        inc! ref! self =
            self.update! old -> old + 1

    c = Counter!.new 1
    c.inc!()
    assert c == 2
    ```

4. 互操作性

    Erg內部與Python兼容，可以零成本導入Python API

    ```python
    # 使用內置的Python模塊
    math, time = pyimport "math", "time"
    {sin; pi} = math
    # 使用外部Python模塊
    tqdm = pyimport "tqdm"

    print! sin pi # 1.2246467991473532e-16
    for! tqdm.tqdm(0..99), i =>
        time.sleep! 0.01 * i
    ```

5. 可讀的錯誤信息

    Erg強調了錯誤信息的可讀性；Erg是一種對程序員友好的語言, ~~不像C++.~~

    ```python
    proc! x =
        l = [1, 2, 3]
        l.push!(x)
        l
    ```

    ```console
    Error[#12]: File example.er, line 3, in <module>::proc!
    2│     l = [1, 2, 3]
    3│     l.push!(x)
             ^^^^^
    AttributeError: Array object has no attribute `.push!`
    hint: to update the internal state of an object, make it mutable by using `!` operator
    hint: `Array` has `push`, see https://erg-lang.github.io/docs/prelude/Array/##push for more information
    hint: `Array!` has `push!`, see https://erg-lang.github.io/docs/prelude/Array!/##push! for more information
    ```

## 要求

[Python3 (3.7~3.11)](https://www.python.org/) 解釋器是必需的。如果計算機上已安裝它，則無需進行任何設置

## 安裝

### 通過Cargo安裝（Rust 包管理器）

```sh
cargo install erg
```

### 從源代碼構建

從源代碼構建需要 Rust 工具鏈

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
cargo build --release
```

### 使用Nix構建

如果你已經安裝了 [Nix](https://nixos.org/), 則以下命令將在項目文件夾 `result/bin/erg` 下生成二進製文件

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix-build
```

如果您已啟用 [Nix Flakes](https://nixos.wiki/wiki/Flakes)

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix build
```

### Flags

通過啟用`--features`標誌，你可以自定義構建和安裝

* 你可以通過`--features {language}`來設置錯誤信息語言

```sh
--features japanese
--features simplified_chinese
--features traditional_chinese
```

還有更多的語言將被加入（我們正在尋找翻譯者。請加入[翻譯項目](./doc/zh_TW/dev_guide/i18n_messages.md)）

* 安裝和構建ELS（Erg語言服務器）
  * `--features els`
* 設置成調試模式（針對貢獻者）
  * `--features debug`
* 完整的REPL體驗
  * `--features full-repl`
* 使顯示效果更好
  * `--features unicode` and `--features pretty`
* 啟用所有功能(除了為語言開發者提供)
  * `features full`
* 參見[這裏](https://github.com/erg-lang/erg/blob/main/doc/zh_TW/dev_guide/build_features.md)了解更多標誌。

## 貢獻

貢獻永遠受到歡迎

想要開始貢獻，請查看 [CONTRIBUTING.md](https://github.com/erg-lang/erg/blob/main/CONTRIBUTING.md)

如果您有任何疑問，請隨時在 [Discord channel](https://discord.gg/zfAAUbgGr4) 上提問

## License

在此存儲庫[assets](./assets)和[doc](./doc)文件夾內的所有文件使用[CC-BY-4.0](./doc/LICENSE)授權。其余文件使用[Apache License 2.0](./LICENSE-APACHE) + [MIT License](./LICENSE-MIT)授權

關於第三方crates的製作人員，請參閱: [THIRD_PARTY_CREDITS.md](./THIRD_PARTY_CREDITS.md)（英文）
