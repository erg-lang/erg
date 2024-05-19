# 在应用程序中嵌入Erg编译器

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/embedding.md%26commit_hash%3Db87c075ffa687802f908f6c394c4a3af9ee6ce16)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/embedding.md&commit_hash=b87c075ffa687802f908f6c394c4a3af9ee6ce16)

在应用程序中嵌入Erg很容易

```toml
[dependencies]
erg = "0.5.12" # 选择最新版本
```

```rust
use erg::DummyVM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vm = DummyVM::default();
    let _res: String = vm.eval("print! \"Hello, world!\"")?;
    Ok(())
}
```

执行需要Python

还有一个不连接到运行时的独立编译器版本

```toml
[dependencies]
erg_compiler = "0.5.12" # 选择最新版本
```

```rust
use erg_compiler::Compiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut compiler = Compiler::default();
    let code = compiler.compile("print!\"Hello, world!\"", "exec")?;
    code.dump_as_pyc("o.pyc", None)?;
    Ok(())
}
```

`Compiler`输出一个名为`CodeObj`的结构。这通常不是很有用，所以你可能想要使用`Transpiler`，它输出一个Python脚本

```rust
use erg_compiler::Transpiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut transpiler = Transpiler::default();
    let script = transpiler.transpile("print!\"Hello, world!\"", "exec")?;
    println!("{}", script.code);
    Ok(())
}
```

其他示例还有输出HIR(高级中间表示)的`HIRBuilder`和输出AST(抽象语法树)的`ASTBuilder`

```rust
use erg_compiler::HIRBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = HIRBuilder::default();
    let artifact = builder.build("print!\"Hello, world!\"", "exec")?;
    println!("{}", artifact.hir);
    Ok(())
}
```

```rust
use erg_compiler::ASTBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = ASTBuilder::default();
    let ast = builder.build("print! \"Hello, world!\")")?;
    println!("{}", ast);
    Ok(())
}
```

If you also want to resolve module dependencies, please use `PackageBuilder`.

```rust
use erg_compiler::PackageBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = PackageBuilder::default();
    let artifact = builder.build("print! \"Hello, world!\"", "exec")?;
    println!("HIR: {}", artifact.object);
    Ok(())
}
```

```rust
use erg_compiler::ASTBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = ASTBuilder::default();
    let ast = builder.build("print! \"Hello, world!\")")?;
    println!("{}", ast);
    Ok(())
}
```

执行语义分析的结构实现了一个名为`ContextProvider`的trait。它可以获取模块中变量的信息，等等

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
