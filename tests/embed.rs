use erg::DummyVM;
use erg_common::error::MultiErrorDisplay;
use erg_compiler::Transpiler;

#[test]
fn test_vm_embedding() -> Result<(), ()> {
    let mut vm = DummyVM::default();
    vm.eval("print! \"Hello, world!\"".into()).map_err(|es| {
        es.fmt_all_stderr();
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
            es.errors.fmt_all_stderr();
        })?;
    assert!(res.object.code.ends_with("(print)(Str(\"\"),)\n"));
    Ok(())
}
