# 在應用程序中嵌入Erg編譯器

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/embedding.md%26commit_hash%3D94185d534afe909d112381b53d60895389d02f95)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/embedding.md&commit_hash=94185d534afe909d112381b53d60895389d02f95)

在應用程序中嵌入Erg很容易

```toml
[dependencies]
erg = "0.5.12" # 選擇最新版本
```

```rust
use erg::DummyVM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vm = DummyVM::default();
    let _res: String = vm.eval("print! \"Hello, world!\"")?;
    Ok(())
}
```

執行需要Python

還有一個不連接到運行時的獨立編譯器版本

```toml
[dependencies]
erg_compiler = "0.5.12" # 選擇最新版本
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

`Compiler`輸出一個名為`CodeObj`的結構。這通常不是很有用，所以你可能想要使用`Transpiler`，它輸出一個Python腳本

```rust
use erg_compiler::Transpiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut transpiler = Transpiler::default();
    let script = transpiler.transpile("print!\"Hello, world!\"", "exec")?;
    println!("{}", script.code);
    Ok(())
}
```

其他示例還有輸出HIR(高級中間表示)的`HIRBuilder`和輸出AST(抽象語法樹)的`ASTBuilder`

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

執行語義分析的結構實現了一個名為`ContextProvider`的trait。它可以獲取模塊中變量的信息，等等

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
