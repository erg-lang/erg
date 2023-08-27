use erg::DummyVM;
use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::exec_py_code_with_output;
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
        .transpile("print!(\"hello\", end:=\"\")".into(), "exec")
        .map_err(|es| {
            es.errors.write_all_stderr();
        })?;
    assert!(res
        .object
        .code()
        .ends_with("(print)(Str(\"hello\"),end=Str(\"\"),)\n"));
    let res = exec_py_code_with_output(res.object.code(), &[]).map_err(|_| ())?;
    assert!(res.status.success());
    assert_eq!(res.stdout, b"hello");
    Ok(())
}

#[test]
fn test_transpiler_embedding2() -> Result<(), ()> {
    let mut trans = Transpiler::default();
    let res = trans
        .transpile(
            "
print!(0, end:=\"\")
i = match [1, 2]:
    [2, 1] -> 0
    [1, 2] -> 1
print!(i, end:=\"\")
j = if False:
    do: 1
    do: 2
print!(j, end:=\"\")
for! [3, 4], i =>
    print!(i, end:=\"\")
c = !5
while! do! c < 7, do!:
    print!(c, end:=\"\")
    c.inc!()
"
            .into(),
            "exec",
        )
        .map_err(|es| {
            es.errors.write_all_stderr();
        })?;
    let res = exec_py_code_with_output(res.object.code(), &[]).map_err(|_| ())?;
    assert!(res.status.success());
    assert_eq!(res.stdout, b"0123456");
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
