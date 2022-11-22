# Ergコンパイラのアプリケーションへの組み込み

Ergには`Compiler`というコンパイラを表現する構造体があり、これを使用することで、Ergのコンパイラを自分のアプリケーションに組み込むことができます。

```toml
[dependencies]
erg_compiler = "0.5.12" # choose latest version
```

```rust
use erg_compiler::Compiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut compiler = Compiler::default();
    let code = compiler.compile("print! \"Hello, world!\"", "exec")?;
    code.dump_as_pyc("o.pyc", None)?;
    Ok(())
}
```

`Compiler`は`CodeObj`という構造体を出力します。これは一般的にはあまり役に立たないので、Pythonのスクリプトを出力する`Transpiler`を使うのも良いでしょう。

```rust
use erg_compiler::Transpiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut transpiler = Transpiler::default();
    let script = transpiler.transpile("print! \"Hello, world!\"", "exec")?;
    println!("{}", script.code);
    Ok(())
}
```

その他にも、HIR(高レベル中間表現)を出力する`HIRBuilder`や、AST(抽象構文木)を出力する`ASTBuilder`もあります。

```rust
use erg_compiler::HIRBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = HIRBuilder::default();
    let artifact = builder.build("print! \"Hello, world!\"", "exec")?;
    println!("{}", artifact.hir);
    Ok(())
}
```

```rust
use erg_compiler::ASTBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = ASTBuilder::default();
    let ast = builder.build("print! \"Hello, world!\"")?;
    println!("{}", ast);
    Ok(())
}
```

構文解析以降の意味解析を行う構造体は、`ContextProvider`というトレイトを実装しています。モジュール内の変数情報を得ることなどができます。

```rust
use erg_compiler::Transpiler;
use erg_compiler::context::ContextProvider;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut transpiler = Transpiler::default();
    let script = transpiler.transpile("i = 0", "exec")?;
    println!("{}", script.code);
    let typ = transpiler.get_var_info("i").0.t;
    println!("{typ}");
    Ok(())
}
```
