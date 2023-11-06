# Embedding the Erg compiler in your application

It is easy to embed Erg in your application.

```toml
[dependencies]
erg = "0.5.12" # choose latest version
```

```rust
use erg::DummyVM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vm = DummyVM::default();
    let _res: String = vm.eval("print! \"Hello, world!\"")?;
    Ok(())
}
```

Python is required for execution.

There is also a stand-alone compiler version that is not connected to the runtime.

```toml
[dependencies]
erg_compiler = "0.5.12" # choose latest version
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

``Compiler`` outputs a structure called `CodeObj`. This is generally not very useful, so you may want to use `Transpiler`, which outputs a Python script.

```rust
use erg_compiler::Transpiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut transpiler = Transpiler::default();
    let script = transpiler.transpile("print!\"Hello, world!\"", "exec")?;
    println!("{}", script.code);
    Ok(())
}
```

Other examples are ``HIRBuilder`` which outputs HIR (high-level intermediate representation) and ``ASTBuilder`` which outputs AST (abstract syntax trees).

```rust
use erg_compiler::HIRBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = HIRBuilder::default();
    let artifact = builder.build("print!\"Hello, world!\"", "exec")?;
    println!("HIR: {}", artifact.object);
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

The structure that performs the semantic analysis implements a trait called `ContextProvider`. It can obtain information about variables in the module, etc.

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
