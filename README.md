# The Erg Programming Language

<div align="center">
    <img width="500" src="./assets/erg_logo_with_slogan.svg">
</div>

<br>This is the main source code repository for [Erg](https://erg-lang.org/). This contains the compiler and documentation.

<p align='center'>
    <a href="https://github.com/erg-lang/erg/releases"><img alt="Build status" src="https://img.shields.io/github/v/release/erg-lang/erg.svg"></a>
    <a href="https://github.com/erg-lang/erg/actions/workflows/rust.yml"><img alt="Build status" src="https://github.com/erg-lang/erg/actions/workflows/rust.yml/badge.svg"></a>
<br>
    <a href="https://erg-lang.org/web-ide/" data-size="large">
        <img src="https://img.shields.io/static/v1?style=for-the-badge&label=&message=playground&color=green">
    </a>
<br>
    <a href='./README_JA.md'>日本語</a> | <a href='./README_zh-CN.md'>简体中文</a> | <a href='./README_zh-TW.md'>繁體中文</a>
</p>

## Erg can be recommended to a person that&colon;

* wants Rust-like robustness and comfortable compiler support, and yet, doesn't need the verbose type specifications & memory management model like Rust.
* frustrated with Python, but can't throw away Python code assets.
* wants a simple and consistent language like ML.
* wants a practical general-purpose language with dependent/refinement types.
* wants a language like Scala that can be used both object-oriented and functional.

## Features

> Some features are not yet implemented. Please see [TODO.md](./TODO.md) for implementation status.

1. Robustness

    Erg has a smart & powerful type system. For example, Erg can do null checking (Option type), division by zero, and out-of-range addresses in arrays at compile time.

    ```python
    rand = pyimport "random"

    l = [1, 2, 3]
    assert l in [Nat; 3] # type checking
    assert l in [1..3; 3] # more detailed
    l2 = l.push(rand.choice! 0..10)
    assert l2 in [0..10; 4]
    assert l2 + [3, 5, 7] in [0..10; 7]
    # This causes an IndexError, Erg can detect it at compile time
    l2[10] # IndexError: `l2` has 7 elements but was accessed the 11th element

    2.times! do!:
        print! "hello, ", end := ""
    # => hello, hello,
    -2.times! do!:
        print! "hello, ", end := ""
    # TypeError: `.times!` is a method of `Nat` (0 or more Int), not `Int`

    {Meter; Sec; meter; yard; sec; ...} = import "unit"

    velocity x: Meter, t: Sec = x / t

    v = velocity 3yard, 2sec # TypeError: the type of `x` was mismatched: expect `Meter`, found `Yard`
    v = velocity 3meter, 2sec # v == 1.5 m/s
    ```

2. Simplicity

    Erg consists of a very simple syntax, which can significantly reduce the amount of code compared to other languages. However, its functionality is not inferior to them.

    Since the type inference system is powerful, you can code like a dynamically typed language.

    ```python
    fib 0 = 0
    fib 1 = 1
    fib n = fib(n - 1) + fib(n - 2)
    assert fib(10) == 55
    ```

    In Erg, there are very few things that are treated as special; there are no reserved words.
    even for and while expressions are just one of the subroutines, so this is possible.

    ```python
    loop! block! = while! do! True, block!

    # equals to `while! do(True), do! print! "hello"`
    loop! do!:
        print! "hello"
    ```

3. Functional & Object-oriented

    Erg is a pure object-oriented language. Everything is an object; types, functions, and operators are all objects. On the other hand, Erg is also a functional language.
    Erg requires some kinds of markers to be placed on code that causes side effects or changes internal state, which can localize the complexity of code. This will greatly improve the maintainability of your code.

    ```python
    # Functional style (immutable), same as `sorted(list)` in Python
    immut_arr = [1, 3, 2]
    assert immut_arr.sort() == [1, 2, 3]
    # Object-oriented style (mutable)
    mut_arr = ![1, 3, 2]
    mut_arr.sort!()
    assert mut_arr == [1, 2, 3]
    i = !1
    i.update! old -> old + 1
    assert i == 2

    # Functions cannot cause side effects
    inc i: Int! =
        i.update! old -> old + 1
    # SyntaxError: cannot call a procedural method in a function
    # hint: only methods of mutable types can change the state of objects

    # Code that uses a lot of side effects is redundant, so you will naturally write pure code
    Counter! = Inherit Int!
    Counter!.
        new i: Int = Self!::__new__ !i
        inc! ref! self =
            self.update! old -> old + 1

    c = Counter!.new 1
    c.inc!()
    assert c == 2
    ```

4. Interoperability

    Erg is internally compatible with Python and can import the Python API at zero cost.

    ```python
    # using built-in Python modules
    math, time = pyimport "math", "time"
    {sin; pi; ...} = math
    # using an external Python module
    Tqdm! = pyimport("tqdm").'tqdm'

    print! sin pi # 1.2246467991473532e-16
    for! Tqdm!.'__call__'(0..99), i =>
        time.sleep! 0.01 * i
    ```

5. Readable Error Messages

    Erg emphasizes the readability of error messages; Erg is a programmer-friendly language, ~~unlike C++.~~

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

## Requirements

A [Python3 (3.7~3.11)](https://www.python.org/) interpreter is required. If it is already installed on your machine, no setup is required.

## Installation

### Installing by cargo (Rust package manager)

```sh
cargo install erg
```

### Building from source

Building from source code requires the Rust toolchain.

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
cargo build --release
```

### Building by Nix

If you've been installed [Nix](https://nixos.org/), the following command will be generate binary into `result/bin/erg` under the project.

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix-build
```

If you've been enabled [Nix Flakes](https://nixos.wiki/wiki/Flakes).

```sh
git clone https://github.com/erg-lang/erg.git
cd erg
nix build
```

### Flags

By enabling the `--features` flag, you can customize the installation and build.

* You can change the language of the error message by using  `--features {language}`

```sh
--features japanese
--features simplified_chinese
--features traditional_chinese
```

And more languages will be added (we are looking for translators. Please join the [Translation Project](./doc/EN/dev_guide/i18n_messages.md)).

* Install and build ELS (Erg Language Server)
  * `--features els`
* Debugging mode (for contributors)
  * `--features debug`
* The full REPL experience
  * `--features full-repl`
* Makes the display look better
  * `--features unicode` and `--features pretty`
* Full features(exclude `--features large_thread`)
  * `--features full`
* See [here](https://github.com/erg-lang/erg/blob/main/doc/EN/dev_guide/build_features.md) for more flags.

## Contribution

Contributions are always welcome!
To get started with contributions, please look [CONTRIBUTING.md](https://github.com/erg-lang/erg/blob/main/CONTRIBUTING.md).

If you have any questions, please feel free to ask them on the [Discord channel](https://discord.gg/zfAAUbgGr4).

## License

All files in the [assets](./assets) and [doc](./doc) folders are licensed with [CC-BY-4.0](./doc/LICENSE). The rest of the files in this repository are licensed with [Apache License 2.0](./LICENSE-APACHE) + [MIT License](./LICENSE-MIT).

For credits about third party crates, see [THIRD_PARTY_CREDITS.md](./THIRD_PARTY_CREDITS.md).
