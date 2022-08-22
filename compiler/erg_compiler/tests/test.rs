use erg_compiler::context::Context;

#[test]
fn test_subtyping() -> Result<(), ()> {
    let context = Context::new_root_module();
    context.test_refinement_subtyping()?;
    Ok(())
}
