use erg_compiler::context::Context;

#[test]
fn test_subtyping() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    context.test_refinement_subtyping()?;
    Ok(())
}

#[test]
fn test_instantiation_and_generalization() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    context.test_instantiation_and_generalization()?;
    Ok(())
}

/*
#[test]
fn test_resolve_trait() -> Result<(), ()> {
    let context = Context::new_main_module();
    context.test_resolve_trait()?;
    Ok(())
}

#[test]
fn test_resolve_trait_inner1() -> Result<(), ()> {
    let context = Context::new_main_module();
    context.test_resolve_trait_inner1()?;
    Ok(())
}
*/

// #[test]
fn _test_dir() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    let vars = context.dir();
    for (name, vi) in vars.into_iter() {
        println!("{name}: {vi}");
    }
    Ok(())
}
