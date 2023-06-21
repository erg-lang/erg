use erg::DummyVM;
use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_compiler::artifact::Buildable;
use erg_compiler::module::SharedCompilerResource;
use erg_compiler::HIRBuilder;
use erg_compiler::Transpiler;

#[test]
fn test_vm_embedding() -> Result<(), ()> {
    let mut vm = DummyVM::default();
    vm.eval("print! \"Hello, world!\"".into()).map_err(|es| {
        es.write_all_stderr();
    })?;
    vm.eval("prin \"Hello, world!\"".into())
        .expect_err("should err");
    Ok(())
}

#[test]
fn test_transpiler_embedding() -> Result<(), ()> {
    let mut trans = Transpiler::default();
    let res = trans
        .transpile("print!(\"\")".into(), "exec")
        .map_err(|es| {
            es.errors.write_all_stderr();
        })?;
    assert!(res.object.code.ends_with("(print)(Str(\"\"),)\n"));
    Ok(())
}

#[test]
fn test_builder() -> Result<(), ()> {
    let mods = ["math", "time"];
    let src = mods.into_iter().fold("".to_string(), |acc, module| {
        acc + &format!("_ = pyimport \"{module}\"\n")
    });
    let cfg = ErgConfig::string(src.clone());
    let shared = SharedCompilerResource::new(cfg.clone());
    let mut checker = HIRBuilder::inherit(cfg, shared);
    let _res = checker.build(src, "exec");
    Ok(())
}
