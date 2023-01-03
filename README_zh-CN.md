# The Erg Programming Language

<div align="center">
    <img width="500" src="./assets/erg_logo_with_slogan.svg">
</div>

<br>这是[Erg](https://erg-lang.org/)的主要源代码库。它包含编译器和文档

<p align='center'>
    <a href="https://github.com/erg-lang/erg/releases"><img alt="Build status" src="https://img.shields.io/github/v/release/erg-lang/erg.svg"></a>
    <a href="https://github.com/erg-lang/erg/actions/workflows/rust.yml"><img alt="Build status" src="https://github.com/erg-lang/erg/actions/workflows/rust.yml/badge.svg"></a>
<br>
    <a href="https://erg-lang.org/web-ide/" data-size="large">
        <img src="https://img.shields.io/static/v1?style=for-the-badge&label=&message=playground&color=green">
    </a>
</p>

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3DREADME.md%26commit_hash%3D54dbd1ec22756e0f8aae5ccf0c41aeb9d34876da)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=README.md&commit_hash=54dbd1ec22756e0f8aae5ccf0c41aeb9d34876da)

## Erg可以推荐给以下人员&colon;

* 希望有类似Rust的健壮性和舒适的编译器支持，然而，又不需要像Rust那样的冗长的类型规范和内存管理模型
* 对Python不满意，但无法下定决心放弃Python代码资产
* 希望有一个像ML那样简单而一致的语言
* 希望有一个实用的通用语言，有依赖/反射类型
* 想要一种像Scala一样的语言，既面向对象，又使用函数式编程

## 特征

> 某些功能尚未实现。有关实施情况请查看 [TODO.md](./TODO.md) 了解实施情况

1. 稳健性

    Erg有一个智能而强大的类型系统。例如: Erg 可以在编译时检查: 空值（Option类型）、除以零的情况、数组中的超出范围的地址

    ```python
    rand = pyimport "random"

    l = [1, 2, 3]
    assert l in [Nat; 3] # 类型检查
    assert l in [1..3; 3] # 更详细
    l2 = l.push(rand.choice! 0..10)
    assert l2 in [0..10; 4]
    assert l2 + [3, 5, 7] in [0..10; 7]
    # 这将导致下标错误，Erg可以在编译时发现它
    l2[10] # 下标错误: `l2`只有7个元素，但却被访问了第11个元素

    2.times! do!:
        print! "hello, ", end := ""
    # => hello, hello,
    -2.times! do!:
        print! "hello, ", end := ""
    # 类型错误: `.times!`是`Nat`(0或更大整数)的方法，不是`Int`的

    {Meter; Sec; meter; yard; sec; ...} = import "unit"

    velocity x: Meter, t: Sec = x / t

    v = velocity 3yard, 2sec # 类型错误: `x`的类型不匹配: 预期为`Meter`，找到`Yard'`
    v = velocity 3meter, 2sec # v == 1.5 m/s
    ```

2. 简洁性

    Erg由一个非常简单的语法组成，与其他语言相比，它可以大大减少代码量。然而，其功能并不逊色于它们

    由于类型推理系统很强大，你可以像动态类型语言一样编码

    ```python
    fib 0 = 0
    fib 1 = 1
    fib n = fib(n - 1) + fib(n - 2)
    assert fib(10) == 55
    ```

    在Erg中，很少有东西被认为是特殊的，没有关键字，因此for和while表达式也只是子程序之一

    ```python
    loop! block = while! do(True), block

    # equals to `while! do(True), do! print! "hello"`
    loop! do!:
        print! "hello"
    ```

3. 函数式 & 面向对象

    Erg是一种纯面向对象的语言，一切都是对象。类型，函数和运算符都是对象。另一方面，Erg也是一种函数式语言
    Erg要求在引起副作用或改变内部状态的代码上放置某些种类的标记，这可以使代码的复杂性局部化，这将大大改善代码的可维护性

    ```python
    # 函数式风格（不可变），与Python中的`sorted(list)`相同
    immut_arr = [1, 3, 2]
    assert immut_arr.sort() == [1, 2, 3]
    # Object-oriented style (mutable)
    mut_arr = ![1, 3, 2]
    mut_arr.sort!()
    assert mut_arr == [1, 2, 3]
    i = !1
    i.update! old -> old + 1
    assert i == 2

    # 函数不能引起副作用
    inc i: Int! =
        i.update! old -> old + 1
    # 语法错误: 不能在函数中调用程序性方法
    # 提示: 只有可变类型的方法才能改变对象的状态

    # 使用大量副作用的代码是多余的，所以你自然会写纯代码
    Counter! = Inherit Int!
    Counter!.
        new i: Int = Self!::__new__ !i
        inc! ref! self =
            self.update! old -> old + 1

    c = Counter!.new 1
    c.inc!()
    assert c == 2
    ```

4. 互操作性

    Erg内部与Python兼容，可以零成本导入Python API

    ```python
    # 使用内置的Python模块
    math, time = pyimport "math", "time"
    {sin; pi; ...} = math
    # 使用外部Python模块
    Tqdm! = pyimport("tqdm").'tqdm'

    print! sin pi # 1.2246467991473532e-16
    for! Tqdm!.'__call__'(0..99), i =>
        time.sleep! 0.01 * i
    ```

5. 可读的错误信息

    Erg强调了错误信息的可读性；Erg是一种对程序员友好的语言, ~~不像C++.~~

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

[Python3 (3.7~3.11)](https://www.python.org/) 解释器是必需的。如果计算机上已安装它，则无需进行任何设置

## 安装

### 通过Cargo安装（Rust 包管理器）

```sh
cargo install erg
```

通过启用`--features`标志，你可以改变显示错误信息的语言

* 日语

```sh
cargo install erg --features japanese
```

* 中文(简体)

```sh
cargo install erg --features simplified_chinese
```

* 中文(繁体)

```sh
cargo install erg --features traditional_chinese
```

还有更多的语言将被加入（我们正在寻找翻译者。请加入[翻译项目](./doc/EN/dev_guide/i18n_messages.md)）

* 调试模式（针对贡献者）

```sh
cargo install erg --features debug
```

### 从源代码构建

从源代码构建需要 Rust 工具链

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
cargo build --release
```

### 使用Nix构建

如果你已经安装了 [Nix](https://nixos.org/), 则以下命令将在项目文件夹 `result/bin/erg` 下生成二进制文件

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix-build
```

如果您已启用 [Nix Flakes](https://nixos.wiki/wiki/Flakes)

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix build
```

## 贡献

贡献永远受到欢迎

想要开始贡献，请查看 [CONTRIBUTING.md](https://github.com/erg-lang/erg/blob/main/CONTRIBUTING.md)

如果您有任何疑问，请随时在 [Discord channel](https://discord.gg/zfAAUbgGr4) 上提问

## License

在此存储库[assets](./assets)和[doc](./doc)文件夹内的所有文件使用[CC-BY-4.0](./doc/LICENSE)授权。其余文件使用[Apache License 2.0](./LICENSE-APACHE) + [MIT License](./LICENSE-MIT)授权

关于第三方crates的制作人员，请参阅: [THIRD_PARTY_CREDITS.md](./THIRD_PARTY_CREDITS.md)（英文）
